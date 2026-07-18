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
    routes::queue,
    services::{audit, import, print_service, printer, settings},
    utils,
    ws::QueueEvent,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/admin/users", get(users).post(create_user))
        .route("/admin/users/import", post(import_users))
        .route("/admin/users/:user_id", delete(delete_user))
        .route("/admin/users/:user_id/reset-password", post(reset_password))
        .route("/admin/users/:user_id/status", post(update_user_status))
        .route("/admin/queue/pause", post(pause_queue))
        .route("/admin/queue/resume", post(resume_queue))
        .route("/admin/tasks/:task_id", delete(cancel_task))
        .route("/admin/review", get(review_tasks))
        .route("/admin/review/:task_id/approve", post(approve_review))
        .route("/admin/review/:task_id/reject", post(reject_review))
        .route("/admin/stats", get(stats))
        .route("/admin/stats.csv", get(stats_csv))
        .route("/admin/config", get(get_config).put(update_config))
        .route("/admin/printer/ack-toner", post(ack_toner))
        .route("/admin/transfer", post(transfer_admin))
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateUserRequest {
    pub student_id: String,
}

pub async fn create_user(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(request): Json<CreateUserRequest>,
) -> AppResult<Json<UserView>> {
    ensure_admin(&user)?;
    let student_id = request.student_id.trim();
    if student_id.is_empty() {
        return Err(AppError::BadRequest(
            "student_id cannot be empty".to_string(),
        ));
    }

    let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE student_id = ?")
        .bind(student_id)
        .fetch_one(&state.pool)
        .await?;
    if exists > 0 {
        return Err(AppError::Conflict("user already exists".to_string()));
    }

    let hash = session::hash_password(student_id)?;
    let result = sqlx::query(
        "INSERT INTO users (student_id, password_hash, role, must_change_password) VALUES (?, ?, 'user', 1)",
    )
    .bind(student_id)
    .bind(hash)
    .execute(&state.pool)
    .await?;
    let created = sqlx::query_as::<_, User>(
        "SELECT id, student_id, password_hash, role, qq, phone, status, must_change_password, created_at, last_login_at FROM users WHERE id = ?",
    )
    .bind(result.last_insert_rowid())
    .fetch_one(&state.pool)
    .await?;

    audit::log(&state.pool, Some(user.id), "admin.users.create", &request).await?;
    Ok(Json(created.into()))
}

#[derive(Debug, Serialize)]
pub struct UsersResponse {
    pub items: Vec<AdminUserView>,
    pub page: i64,
    pub per_page: i64,
    pub total: i64,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AdminUserView {
    pub id: i64,
    pub student_id: String,
    pub role: String,
    pub qq: Option<String>,
    pub phone: Option<String>,
    pub status: String,
    pub must_change_password: bool,
    pub created_at: String,
    pub last_login_at: Option<String>,
    pub total_pages: i64,
    pub total_tasks: i64,
}

#[derive(Debug, Deserialize)]
pub struct UsersQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub q: Option<String>,
    pub role: Option<String>,
    pub status: Option<String>,
}

pub async fn users(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Query(query): Query<UsersQuery>,
) -> AppResult<Json<UsersResponse>> {
    ensure_admin(&user)?;
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(50).clamp(1, 200);
    let offset = (page - 1) * per_page;
    let search = query
        .q
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| format!("%{value}%"));
    let role = validated_filter(query.role.as_deref(), &["admin", "user"], "角色")?;
    let status = validated_filter(
        query.status.as_deref(),
        &["normal", "banned", "unused"],
        "状态",
    )?;

    let total: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM users
        WHERE (? IS NULL OR role = ?)
          AND (? IS NULL OR status = ?)
          AND (? IS NULL OR student_id LIKE ?)
        "#,
    )
    .bind(role)
    .bind(role)
    .bind(status)
    .bind(status)
    .bind(search.as_deref())
    .bind(search.as_deref())
    .fetch_one(&state.pool)
    .await?;
    let items = sqlx::query_as::<_, AdminUserView>(
        r#"
        SELECT u.id, u.student_id, u.role, u.qq, u.phone, u.status, u.must_change_password,
               u.created_at, u.last_login_at,
               COALESCE(SUM(CASE WHEN t.status = 'done' THEN t.page_count ELSE 0 END), 0) AS total_pages,
               COALESCE(SUM(CASE WHEN t.status = 'done' THEN 1 ELSE 0 END), 0) AS total_tasks
        FROM users u
        LEFT JOIN print_tasks t ON t.user_id = u.id
        WHERE (? IS NULL OR u.role = ?)
          AND (? IS NULL OR u.status = ?)
          AND (? IS NULL OR u.student_id LIKE ?)
        GROUP BY u.id
        ORDER BY u.role = 'admin' DESC, u.student_id ASC
        LIMIT ? OFFSET ?
        "#,
    )
    .bind(role)
    .bind(role)
    .bind(status)
    .bind(status)
    .bind(search.as_deref())
    .bind(search.as_deref())
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(UsersResponse {
        items,
        page,
        per_page,
        total,
    }))
}

fn validated_filter<'a>(
    value: Option<&'a str>,
    allowed: &[&str],
    label: &str,
) -> AppResult<Option<&'a str>> {
    let value = value.map(str::trim).filter(|value| !value.is_empty());
    if value.is_some_and(|value| !allowed.contains(&value)) {
        return Err(AppError::BadRequest(format!("无效的{label}筛选条件")));
    }
    Ok(value)
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
    import::ensure_supported_file_name(&file_name)?;
    let bytes = field.bytes().await?;
    if bytes.len() as u64 > state.config.limits.max_import_bytes {
        return Err(AppError::BadRequest("用户导入文件过大".into()));
    }
    let bytes = bytes.to_vec();
    let path = utils::tmp_dir(&state.config).join(format!(
        "{}_{}",
        Uuid::new_v4(),
        utils::sanitize_filename(&file_name)
    ));
    fs::write(&path, &bytes).await?;

    let student_ids = match import::parse_student_ids(&path, &bytes) {
        Ok(ids) => ids,
        Err(error) => {
            let _ = fs::remove_file(&path).await;
            return Err(error);
        }
    };
    let _ = fs::remove_file(&path).await;
    let mut created = Vec::new();
    let mut skipped = Vec::new();
    let hashes = tokio::task::spawn_blocking(move || {
        student_ids
            .into_iter()
            .map(|student_id| {
                let hash = session::hash_password(&student_id)?;
                Ok((student_id, hash))
            })
            .collect::<AppResult<Vec<_>>>()
    })
    .await
    .map_err(|error| AppError::External(format!("用户导入任务失败：{error}")))??;
    let mut tx = state.pool.begin().await?;
    for (student_id, hash) in hashes {
        let result = sqlx::query(
            "INSERT OR IGNORE INTO users (student_id, password_hash, role, must_change_password) VALUES (?, ?, 'user', 1)",
        )
        .bind(&student_id)
        .bind(hash)
        .execute(&mut *tx)
        .await?;
        if result.rows_affected() == 1 {
            created.push(student_id);
        } else {
            skipped.push(student_id);
        }
    }
    tx.commit().await?;

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
    let _queue_guard = state.queue_lock.lock().await;

    let printing_jobs = sqlx::query_as::<_, (i64, Option<i64>)>(
        "SELECT id, windows_job_id FROM print_tasks WHERE user_id = ? AND status = 'printing'",
    )
    .bind(user_id)
    .fetch_all(&state.pool)
    .await?;
    for (task_id, job_id) in printing_jobs {
        let job_id = job_id.ok_or_else(|| {
            AppError::Conflict(format!(
                "任务 {task_id} 尚未记录 Windows 作业 ID，无法安全删除用户"
            ))
        })?;
        printer::cancel_job(&state.config, job_id).await?;
    }

    let paths = file_paths_for_user(&state.pool, user_id).await?;
    sqlx::query(
        "UPDATE print_tasks SET status = 'cancelled', cancelled_by = 'admin', completed_at = datetime('now') WHERE user_id = ? AND status IN ('queued', 'printing', 'pending_review')",
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
pub struct ResetPasswordRequest {}

pub async fn reset_password(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(user_id): Path<i64>,
    Json(_request): Json<ResetPasswordRequest>,
) -> AppResult<Json<serde_json::Value>> {
    ensure_admin(&user)?;
    let target = sqlx::query_as::<_, User>(
        "SELECT id, student_id, password_hash, role, qq, phone, status, must_change_password, created_at, last_login_at FROM users WHERE id = ?",
    )
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("user not found".to_string()))?;

    let hash = session::hash_password(&target.student_id)?;
    sqlx::query("UPDATE users SET password_hash = ?, must_change_password = 1 WHERE id = ?")
        .bind(hash)
        .bind(user_id)
        .execute(&state.pool)
        .await?;
    session::delete_user_sessions(&state.pool, user_id).await?;

    audit::log(
        &state.pool,
        Some(user.id),
        "admin.users.reset_password",
        &serde_json::json!({ "user_id": user_id }),
    )
    .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateUserStatusRequest {
    pub status: String,
}

pub async fn update_user_status(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(user_id): Path<i64>,
    Json(request): Json<UpdateUserStatusRequest>,
) -> AppResult<Json<serde_json::Value>> {
    ensure_admin(&user)?;
    if user.id == user_id {
        return Err(AppError::Conflict(
            "不能封禁或解封当前管理员账号".to_string(),
        ));
    }
    if request.status != "banned" && request.status != "normal" {
        return Err(AppError::BadRequest(
            "status must be banned or normal".to_string(),
        ));
    }

    let target = sqlx::query_as::<_, (String, String, Option<String>)>(
        "SELECT student_id, status, last_login_at FROM users WHERE id = ?",
    )
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("user not found".to_string()))?;

    let next_status = if request.status == "banned" {
        "banned"
    } else if target.2.is_none() {
        "unused"
    } else {
        "normal"
    };

    sqlx::query("UPDATE users SET status = ? WHERE id = ?")
        .bind(next_status)
        .bind(user_id)
        .execute(&state.pool)
        .await?;
    if next_status == "banned" {
        session::delete_user_sessions(&state.pool, user_id).await?;
    }

    audit::log(
        &state.pool,
        Some(user.id),
        "admin.users.status",
        &serde_json::json!({
            "user_id": user_id,
            "student_id": target.0,
            "previous_status": target.1,
            "status": next_status
        }),
    )
    .await?;

    Ok(Json(
        serde_json::json!({ "ok": true, "status": next_status }),
    ))
}

pub async fn pause_queue(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Json<serde_json::Value>> {
    ensure_admin(&user)?;
    settings::set_queue_paused(&state.pool, true).await?;
    audit::log(
        &state.pool,
        Some(user.id),
        "admin.queue.pause",
        &serde_json::json!({}),
    )
    .await?;
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
    audit::log(
        &state.pool,
        Some(user.id),
        "admin.queue.resume",
        &serde_json::json!({}),
    )
    .await?;
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
    let _queue_guard = state.queue_lock.lock().await;
    let reason = body.and_then(|Json(body)| body.reason);
    let current = print_service::load_task(&state.pool, task_id).await?;
    let task = if current.status == "printing" {
        let job_id = current.windows_job_id.ok_or_else(|| {
            AppError::Conflict(
                "该任务尚未记录系统作业 ID，无法安全取消；请先检查系统打印队列".to_string(),
            )
        })?;
        printer::cancel_job(&state.config, job_id).await?;
        sqlx::query("UPDATE print_tasks SET status = 'cancelled', cancelled_by = 'admin', review_reason = ?, completed_at = datetime('now'), status_detail = '系统打印作业已取消' WHERE id = ? AND status = 'printing'")
            .bind(reason).bind(task_id).execute(&state.pool).await?;
        if let Some(preview) = current.preview_path.as_deref() {
            if let Some(parent) = std::path::Path::new(preview).parent() {
                let _ = fs::remove_file(parent.join(format!("print-task-{task_id}.pdf"))).await;
            }
        }
        print_service::load_task(&state.pool, task_id).await?
    } else {
        print_service::cancel_task(&state.pool, task_id, &user, reason).await?
    };
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
               t.status, t.submitted_at, t.completed_at, t.review_reason, t.status_detail,
               t.submitted_ip, t.windows_job_id,
               CASE WHEN t.preview_path IS NOT NULL AND t.preview_path != '' THEN 1 ELSE 0 END AS preview_available
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
            .map(|row| queue::to_view(row, &user))
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
            queued_at = datetime('now'),
            reviewed_at = datetime('now'),
            reviewed_by = ?,
            review_reason = NULL
        WHERE id = ? AND status = 'pending_review'
        "#,
    )
    .bind(user.id)
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
        SET status = 'cancelled', cancelled_by = 'admin', review_reason = ?,
            completed_at = datetime('now'), reviewed_at = datetime('now'), reviewed_by = ?
        WHERE id = ? AND status = 'pending_review'
        "#,
    )
    .bind(request.reason)
    .bind(user.id)
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
    pub queue_paused: bool,
    pub printer: crate::services::printer::PrinterState,
}

pub async fn get_config(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Json<ConfigResponse>> {
    ensure_admin(&user)?;
    Ok(Json(ConfigResponse {
        daily_limit: settings::get_or(&state.pool, "daily_limit", "10").await?,
        queue_paused: settings::queue_paused(&state.pool).await?,
        printer: state.printer_state.read().await.clone(),
    }))
}

pub async fn ack_toner(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Json<serde_json::Value>> {
    ensure_admin(&user)?;
    let mut printer = state.printer_state.write().await;
    printer.toner_alert_acknowledged = true;
    Ok(Json(serde_json::json!({ "ok": true })))
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

    let new_admin = sqlx::query_as::<_, User>(
        "SELECT id, student_id, password_hash, role, qq, phone, status, must_change_password, created_at, last_login_at FROM users WHERE student_id = ?",
    )
    .bind(student_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("接任管理员账号不存在，请先添加用户".to_string()))?;

    if new_admin.id == user.id {
        return Err(AppError::Conflict("请选择其他账号接任管理员".to_string()));
    }
    if new_admin.status == "banned" {
        return Err(AppError::Conflict(
            "不能将管理员转让给已封禁账号".to_string(),
        ));
    }

    let mut tx = state.pool.begin().await?;
    sqlx::query("UPDATE users SET role = 'user'")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE users SET role = 'admin' WHERE id = ?")
        .bind(new_admin.id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

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
        id: i64,
        stored_path: Option<String>,
        preview_path: Option<String>,
    }

    let task_paths = sqlx::query_as::<_, PathRow>(
        "SELECT id, stored_path, preview_path FROM print_tasks WHERE user_id = ?",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    let upload_paths = sqlx::query_as::<_, PathRow>(
        "SELECT -1 AS id, stored_path, preview_path FROM temp_uploads WHERE user_id = ?",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(task_paths
        .into_iter()
        .chain(upload_paths)
        .flat_map(|row| {
            let spool = (row.id > 0)
                .then(|| {
                    row.preview_path.as_deref().and_then(|preview| {
                        PathBuf::from(preview)
                            .parent()
                            .map(|parent| parent.join(format!("print-task-{}.pdf", row.id)))
                    })
                })
                .flatten();
            [
                row.stored_path.map(PathBuf::from),
                row.preview_path.map(PathBuf::from),
                spool,
            ]
        })
        .flatten()
        .collect())
}
