use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::{
    app::AppState,
    auth::middleware::CurrentUser,
    db::models::User,
    error::AppResult,
    services::{printer::PrinterState, settings},
};

pub fn router() -> Router<AppState> {
    Router::new().route("/queue", get(queue))
}

#[derive(Debug, Deserialize)]
pub struct QueueQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    #[serde(default)]
    pub mine_only: bool,
    pub student_id: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct QueueRow {
    pub id: i64,
    pub user_id: i64,
    pub student_id: String,
    pub file_name: String,
    pub page_count: i64,
    pub odd_even: String,
    pub status: String,
    pub submitted_at: String,
    pub completed_at: Option<String>,
    pub review_reason: Option<String>,
    pub status_detail: Option<String>,
    pub submitted_ip: Option<String>,
    pub windows_job_id: Option<i64>,
    pub preview_available: bool,
}

#[derive(Debug, Serialize)]
pub struct QueueTaskView {
    pub id: i64,
    pub status: String,
    pub page_count: i64,
    pub odd_even: String,
    pub submitted_at: String,
    pub completed_at: Option<String>,
    pub owner_name: Option<String>,
    pub mine: bool,
    pub file_name_visible: bool,
    pub file_name: Option<String>,
    pub review_reason: Option<String>,
    pub status_detail: Option<String>,
    pub submitted_ip: Option<String>,
    pub windows_job_id: Option<i64>,
    pub preview_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct QueueResponse {
    pub tasks: Vec<QueueTaskView>,
    pub page: i64,
    pub per_page: i64,
    pub total: i64,
    pub paused: bool,
    pub printer: PrinterState,
}

pub async fn queue(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Query(query): Query<QueueQuery>,
) -> AppResult<Json<QueueResponse>> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(50).clamp(1, 200);
    let student_id = query
        .student_id
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let (rows, total) = queue_rows(
        &state.pool,
        &user,
        query.mine_only,
        student_id.as_deref(),
        page,
        per_page,
    )
    .await?;
    let paused = settings::queue_paused(&state.pool).await?;
    Ok(Json(QueueResponse {
        tasks: rows.into_iter().map(|row| to_view(row, &user)).collect(),
        page,
        per_page,
        total,
        paused,
        printer: state.printer_state.read().await.clone(),
    }))
}

pub async fn queue_rows(
    pool: &sqlx::SqlitePool,
    user: &User,
    mine_only: bool,
    student_id: Option<&str>,
    page: i64,
    per_page: i64,
) -> AppResult<(Vec<QueueRow>, i64)> {
    let offset = (page - 1) * per_page;
    let student_filter = student_id.filter(|_| !mine_only);
    // The queue is shared: every signed-in user can inspect the retained
    // queue history, and can narrow it by student id.
    let visibility = if mine_only {
        "t.user_id = ?"
    } else if student_filter.is_some() {
        "u.student_id = ?"
    } else {
        "1 = 1"
    };
    let count_sql = format!(
        "SELECT COUNT(*) FROM print_tasks t JOIN users u ON u.id = t.user_id WHERE {visibility}"
    );
    let list_sql = format!(
        r#"
        SELECT t.id, t.user_id, u.student_id, t.file_name, t.page_count, t.odd_even,
               t.status, t.submitted_at, t.completed_at, t.review_reason, t.status_detail,
               t.submitted_ip, t.windows_job_id,
               CASE WHEN t.preview_path IS NOT NULL AND t.preview_path != '' THEN 1 ELSE 0 END AS preview_available
        FROM print_tasks t
        JOIN users u ON u.id = t.user_id
        WHERE {visibility}
        ORDER BY
            CASE t.status WHEN 'printing' THEN 0 WHEN 'queued' THEN 1 WHEN 'pending_review' THEN 2 ELSE 3 END,
            CASE WHEN t.status IN ('queued', 'printing', 'pending_review') THEN t.submitted_at END ASC,
            t.id DESC
        LIMIT ? OFFSET ?
        "#
    );

    let total = if let Some(student_id) = student_filter {
        sqlx::query_scalar(&count_sql)
            .bind(student_id)
            .fetch_one(pool)
            .await?
    } else if mine_only {
        sqlx::query_scalar(&count_sql)
            .bind(user.id)
            .fetch_one(pool)
            .await?
    } else {
        sqlx::query_scalar(&count_sql).fetch_one(pool).await?
    };
    let rows = if let Some(student_id) = student_filter {
        sqlx::query_as::<_, QueueRow>(&list_sql)
            .bind(student_id)
            .bind(per_page)
            .bind(offset)
            .fetch_all(pool)
            .await?
    } else if mine_only {
        sqlx::query_as::<_, QueueRow>(&list_sql)
            .bind(user.id)
            .bind(per_page)
            .bind(offset)
            .fetch_all(pool)
            .await?
    } else {
        sqlx::query_as::<_, QueueRow>(&list_sql)
            .bind(per_page)
            .bind(offset)
            .fetch_all(pool)
            .await?
    };
    Ok((rows, total))
}

pub fn to_view(row: QueueRow, user: &User) -> QueueTaskView {
    let mine = row.user_id == user.id;
    QueueTaskView {
        id: row.id,
        status: row.status,
        page_count: row.page_count,
        odd_even: row.odd_even,
        submitted_at: row.submitted_at,
        completed_at: row.completed_at,
        owner_name: Some(row.student_id),
        mine,
        file_name_visible: true,
        file_name: Some(row.file_name),
        review_reason: row.review_reason,
        status_detail: row.status_detail,
        submitted_ip: row.submitted_ip,
        windows_job_id: row.windows_job_id,
        preview_url: row
            .preview_available
            .then(|| format!("/api/print/tasks/{}/preview", row.id)),
    }
}
