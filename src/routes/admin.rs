use std::path::PathBuf;

use axum::{
    extract::{Multipart, Path, Query, State},
    http::{header, HeaderMap, HeaderValue},
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tokio::fs;
use uuid::Uuid;

use crate::{
    app::AppState,
    auth::{ensure_admin, middleware::CurrentUser, session},
    db::models::{PrintTask, User, UserView},
    error::{AppError, AppResult},
    routes::{history::PageQuery, queue},
    services::{audit, import, print_service, printer, settings},
    utils::file,
    ws::QueueEvent,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/admin/users", get(users))
        .route("/admin/users/import", post(import_users))
        .route("/admin/users/:user_id", delete(delete_user))
        .route("/admin/users/:user_id/reset-password", post(reset_password))
        .route("/admin/queue", get(admin_queue))
        .route("/admin/queue/pause", post(pause_queue))
        .route("/admin/queue/resume", post(resume_queue))
        .route("/admin/tasks/:task_id", delete(cancel_task))
        .route("/admin/review", get(review_tasks))
        .route("/admin/review/:task_id/approve", post(approve_review))
        .route("/admin/review/:task_id/reject", post(reject_review))
        .route("/admin/stats", get(stats))
        .route("/admin/stats.csv", get(stats_csv))
        .route("/admin/config", get(get_config).put(update_config))
        .route("/admin/transfer", post(transfer_admin))
}

#[derive(Debug, Serialize)]
pub struct UsersResponse {
    pub items: Vec<UserView>,
    pub page: i64,
    pub per_page: i64,
    pub total: i64,
}

pub async fn users(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Query(query): Query<PageQuery>,
) -> AppResult<Json<UsersResponse>> {
    ensure_admin(&user)?;
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(50).clamp(1, 200);
    let offset = (page - 1) * per_page;

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&state.pool)
        .await?;
    let items = sqlx::query_as::<_, User>(
        r#"
        SELECT id, student_id, password_hash, role, qq, must_change_password, created_at
        FROM users
        ORDER BY role = 'admin' DESC, student_id ASC
        LIMIT ? OFFSET ?
        "#,
    )
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?
    .into_iter()
    .map(UserView::from)
    .collect();

    Ok(Json(UsersResponse {
        items,
        page,
        per_page,
        total,
    }))
}

#[derive(Debug, Serialize)]
pub struct ImportResponse {
    pub created: Vec<String>,
    pub skipped: Vec<String>,
}

pub async fn import_users(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    mut multipart: Multipart,
) -> AppResult<Json<ImportResponse>> {
    ensure_admin(&user)?;

    let Some(field) = multipart.next_field().await? else {
        return Err(AppError::BadRequest("missing import file".to_string()));
    };
    let file_name = field.file_name().unwrap_or("users.txt").to_string();
    let bytes = field.bytes().await?.to_vec();
    let path = file::tmp_dir(&state.config).join(format!(
        "{}_{}",
        Uuid::new_v4(),
        file::sanitize_filename(&file_name)
    ));
    fs::write(&path, &bytes).await?;

    let student_ids = import::parse_student_ids(&path, &bytes)?;
    let mut created = Vec::new();
    let mut skipped = Vec::new();

    for student_id in student_ids {
        let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE student_id = ?")
            .bind(&student_id)
            .fetch_one(&state.pool)
            .await?;
        if exists > 0 {
            skipped.push(student_id);
            continue;
        }

        let hash = session::hash_password(&student_id)?;
        sqlx::query(
            "INSERT INTO users (student_id, password_hash, role, must_change_password) VALUES (?, ?, 'user', 1)",
        )
        .bind(&student_id)
        .bind(hash)
        .execute(&state.pool)
        .await?;
        created.push(student_id);
    }

    let response = ImportResponse { created, skipped };
    audit::log(&state.pool, Some(user.id), "admin.users.import", &response).await?;
    Ok(Json(response))
}

pub async fn delete_user(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(user_id): Path<i64>,
) -> AppResult<Json<serde_json::Value>> {
    ensure_admin(&user)?;
    if user.id == user_id {
        return Err(AppError::Conflict(
            "admin cannot delete the current account".to_string(),
        ));
    }

    let paths = file_paths_for_user(&state.pool, user_id).await?;
    sqlx::query(
        "UPDATE print_tasks SET status = 'cancelled', cancelled_by = 'admin' WHERE user_id = ? AND status IN ('queued', 'printing', 'pending_review')",
    )
    .bind(user_id)
    .execute(&state.pool)
    .await?;
    sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(user_id)
        .execute(&state.pool)
        .await?;

    for path in paths {
        let _ = fs::remove_file(path).await;
    }

    audit::log(
        &state.pool,
        Some(user.id),
        "admin.users.delete",
        &serde_json::json!({ "user_id": user_id }),
    )
    .await?;
    state.broadcaster.send(QueueEvent::QueueChanged);
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Debug, Deserialize)]
pub struct ResetPasswordRequest {
    pub new_password: Option<String>,
}

pub async fn reset_password(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(user_id): Path<i64>,
    Json(request): Json<ResetPasswordRequest>,
) -> AppResult<Json<serde_json::Value>> {
    ensure_admin(&user)?;
    let target = sqlx::query_as::<_, User>(
        "SELECT id, student_id, password_hash, role, qq, must_change_password, created_at FROM users WHERE id = ?",
    )
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("user not found".to_string()))?;

    let new_password = request
        .new_password
        .filter(|password| !password.trim().is_empty())
        .unwrap_or_else(|| target.student_id.clone());
    let hash = session::hash_password(&new_password)?;
    sqlx::query("UPDATE users SET password_hash = ?, must_change_password = 1 WHERE id = ?")
        .bind(hash)
        .bind(user_id)
        .execute(&state.pool)
        .await?;

    audit::log(
        &state.pool,
        Some(user.id),
        "admin.users.reset_password",
        &serde_json::json!({ "user_id": user_id }),
    )
    .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn admin_queue(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Json<queue::QueueResponse>> {
    ensure_admin(&user)?;
    let rows = queue::queue_rows(&state.pool).await?;
    let paused = settings::queue_paused(&state.pool).await?;
    Ok(Json(queue::QueueResponse {
        tasks: rows
            .into_iter()
            .map(|row| queue::to_view(row, &user, true))
            .collect(),
        paused,
    }))
}

pub async fn pause_queue(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Json<serde_json::Value>> {
    ensure_admin(&user)?;
    settings::set_queue_paused(&state.pool, true).await?;
    state
        .broadcaster
        .send(QueueEvent::QueuePaused { paused: true });
    Ok(Json(serde_json::json!({ "paused": true })))
}

pub async fn resume_queue(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Json<serde_json::Value>> {
    ensure_admin(&user)?;
    settings::set_queue_paused(&state.pool, false).await?;
    state
        .broadcaster
        .send(QueueEvent::QueuePaused { paused: false });
    state.broadcaster.send(QueueEvent::QueueChanged);
    Ok(Json(serde_json::json!({ "paused": false })))
}

#[derive(Debug, Deserialize)]
pub struct CancelRequest {
    pub reason: Option<String>,
}

pub async fn cancel_task(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(task_id): Path<i64>,
    body: Option<Json<CancelRequest>>,
) -> AppResult<Json<PrintTask>> {
    ensure_admin(&user)?;
    let reason = body.and_then(|Json(body)| body.reason);
    let task = print_service::cancel_task(&state.pool, task_id, &user, reason).await?;
    audit::log(&state.pool, Some(user.id), "admin.tasks.cancel", &task).await?;
    state.broadcaster.send(QueueEvent::QueueChanged);
    Ok(Json(task))
}

pub async fn review_tasks(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Json<Vec<queue::QueueTaskView>>> {
    ensure_admin(&user)?;
    let rows = sqlx::query_as::<_, queue::QueueRow>(
        r#"
        SELECT t.id, t.user_id, u.student_id, t.file_name, t.page_count, t.odd_even,
               t.status, t.submitted_at, t.review_reason
        FROM print_tasks t
        JOIN users u ON u.id = t.user_id
        WHERE t.status = 'pending_review'
        ORDER BY t.id ASC
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(
        rows.into_iter()
            .map(|row| queue::to_view(row, &user, true))
            .collect(),
    ))
}

pub async fn approve_review(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(task_id): Path<i64>,
) -> AppResult<Json<serde_json::Value>> {
    ensure_admin(&user)?;
    let affected = sqlx::query(
        r#"
        UPDATE print_tasks
        SET status = 'queued',
            approved_over_quota = 1,
            submitted_at = datetime('now'),
            review_reason = NULL
        WHERE id = ? AND status = 'pending_review'
        "#,
    )
    .bind(task_id)
    .execute(&state.pool)
    .await?
    .rows_affected();

    if affected == 0 {
        return Err(AppError::NotFound(
            "pending review task not found".to_string(),
        ));
    }

    audit::log(
        &state.pool,
        Some(user.id),
        "admin.review.approve",
        &serde_json::json!({ "task_id": task_id }),
    )
    .await?;
    state.broadcaster.send(QueueEvent::QueueChanged);
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Debug, Deserialize)]
pub struct RejectRequest {
    pub reason: Option<String>,
}

pub async fn reject_review(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(task_id): Path<i64>,
    Json(request): Json<RejectRequest>,
) -> AppResult<Json<serde_json::Value>> {
    ensure_admin(&user)?;
    let affected = sqlx::query(
        r#"
        UPDATE print_tasks
        SET status = 'cancelled', cancelled_by = 'admin', review_reason = ?
        WHERE id = ? AND status = 'pending_review'
        "#,
    )
    .bind(request.reason)
    .bind(task_id)
    .execute(&state.pool)
    .await?
    .rows_affected();

    if affected == 0 {
        return Err(AppError::NotFound(
            "pending review task not found".to_string(),
        ));
    }

    audit::log(
        &state.pool,
        Some(user.id),
        "admin.review.reject",
        &serde_json::json!({ "task_id": task_id }),
    )
    .await?;
    state.broadcaster.send(QueueEvent::QueueChanged);
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct StatRow {
    pub student_id: String,
    pub total_pages: i64,
    pub total_tasks: i64,
}

pub async fn stats(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Json<Vec<StatRow>>> {
    ensure_admin(&user)?;
    Ok(Json(load_stats(&state.pool).await?))
}

pub async fn stats_csv(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<impl IntoResponse> {
    ensure_admin(&user)?;
    let rows = load_stats(&state.pool).await?;
    let mut csv = String::from("student_id,total_pages,total_tasks\n");
    for row in rows {
        csv.push_str(&format!(
            "{},{},{}\n",
            csv_cell(&row.student_id),
            row.total_pages,
            row.total_tasks
        ));
    }

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/csv; charset=utf-8"),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=\"print-stats.csv\""),
    );
    Ok((headers, csv))
}

async fn load_stats(pool: &sqlx::SqlitePool) -> AppResult<Vec<StatRow>> {
    let rows = sqlx::query_as::<_, StatRow>(
        r#"
        SELECT u.student_id,
               COALESCE(SUM(CASE WHEN t.status = 'done' THEN t.page_count ELSE 0 END), 0) AS total_pages,
               COALESCE(SUM(CASE WHEN t.status = 'done' THEN 1 ELSE 0 END), 0) AS total_tasks
        FROM users u
        LEFT JOIN print_tasks t ON t.user_id = u.id
        GROUP BY u.id
        ORDER BY total_pages DESC, total_tasks DESC, u.student_id ASC
        "#,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

fn csv_cell(value: &str) -> String {
    if value
        .chars()
        .any(|ch| matches!(ch, ',' | '"' | '\n' | '\r'))
    {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

#[derive(Debug, Serialize)]
pub struct ConfigResponse {
    pub daily_limit: String,
    pub admin_qq: String,
    pub admin_student_id: String,
    pub queue_paused: bool,
    pub printer_status: String,
    pub install_hint: &'static str,
}

pub async fn get_config(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Json<ConfigResponse>> {
    ensure_admin(&user)?;
    Ok(Json(ConfigResponse {
        daily_limit: settings::get_or(&state.pool, "daily_limit", "50").await?,
        admin_qq: settings::get_or(&state.pool, "admin_qq", "").await?,
        admin_student_id: settings::get_or(&state.pool, "admin_student_id", "").await?,
        queue_paused: settings::queue_paused(&state.pool).await?,
        printer_status: printer::status(&state.config).await,
        install_hint: crate::utils::windows_service::install_hint(),
    }))
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateConfigRequest {
    pub key: String,
    pub value: String,
}

pub async fn update_config(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(request): Json<UpdateConfigRequest>,
) -> AppResult<Json<serde_json::Value>> {
    ensure_admin(&user)?;
    match request.key.as_str() {
        "daily_limit" => {
            let limit = request
                .value
                .parse::<i64>()
                .map_err(|_| AppError::BadRequest("daily_limit must be a number".to_string()))?;
            settings::set(&state.pool, "daily_limit", &limit.max(0).to_string()).await?;
        }
        "admin_qq" | "admin_student_id" => {
            settings::set(&state.pool, &request.key, request.value.trim()).await?;
        }
        _ => {
            return Err(AppError::BadRequest("unsupported config key".to_string()));
        }
    }

    audit::log(&state.pool, Some(user.id), "admin.config.update", &request).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Debug, Deserialize)]
pub struct TransferRequest {
    pub new_admin_student_id: String,
}

pub async fn transfer_admin(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(request): Json<TransferRequest>,
) -> AppResult<Json<serde_json::Value>> {
    ensure_admin(&user)?;
    let student_id = request.new_admin_student_id.trim();
    if student_id.is_empty() {
        return Err(AppError::BadRequest(
            "new_admin_student_id cannot be empty".to_string(),
        ));
    }

    let existing = sqlx::query_as::<_, User>(
        "SELECT id, student_id, password_hash, role, qq, must_change_password, created_at FROM users WHERE student_id = ?",
    )
    .bind(student_id)
    .fetch_optional(&state.pool)
    .await?;

    let new_admin = if let Some(user) = existing {
        user
    } else {
        let hash = session::hash_password(student_id)?;
        let result = sqlx::query(
            "INSERT INTO users (student_id, password_hash, role, must_change_password) VALUES (?, ?, 'user', 1)",
        )
        .bind(student_id)
        .bind(hash)
        .execute(&state.pool)
        .await?;
        sqlx::query_as::<_, User>(
            "SELECT id, student_id, password_hash, role, qq, must_change_password, created_at FROM users WHERE id = ?",
        )
        .bind(result.last_insert_rowid())
        .fetch_one(&state.pool)
        .await?
    };

    sqlx::query("UPDATE users SET role = 'user'")
        .execute(&state.pool)
        .await?;
    sqlx::query("UPDATE users SET role = 'admin' WHERE id = ?")
        .bind(new_admin.id)
        .execute(&state.pool)
        .await?;
    settings::set(&state.pool, "admin_student_id", &new_admin.student_id).await?;
    if let Some(qq) = new_admin.qq.as_deref() {
        settings::set(&state.pool, "admin_qq", qq).await?;
    }

    audit::log(
        &state.pool,
        Some(user.id),
        "admin.transfer",
        &serde_json::json!({ "new_admin": student_id }),
    )
    .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn file_paths_for_user(pool: &sqlx::SqlitePool, user_id: i64) -> AppResult<Vec<PathBuf>> {
    #[derive(sqlx::FromRow)]
    struct PathRow {
        stored_path: Option<String>,
        preview_path: Option<String>,
    }

    let task_paths = sqlx::query_as::<_, PathRow>(
        "SELECT stored_path, preview_path FROM print_tasks WHERE user_id = ?",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    let upload_paths = sqlx::query_as::<_, PathRow>(
        "SELECT stored_path, preview_path FROM temp_uploads WHERE user_id = ?",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(task_paths
        .into_iter()
        .chain(upload_paths)
        .flat_map(|row| [row.stored_path, row.preview_path])
        .flatten()
        .map(PathBuf::from)
        .collect())
}
