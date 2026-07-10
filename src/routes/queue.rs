use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;

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
    pub review_reason: Option<String>,
    pub status_detail: Option<String>,
    pub windows_job_id: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct QueueTaskView {
    pub id: i64,
    pub status: String,
    pub page_count: i64,
    pub odd_even: String,
    pub submitted_at: String,
    pub owner_name: Option<String>,
    pub mine: bool,
    pub file_name_visible: bool,
    pub file_name: Option<String>,
    pub review_reason: Option<String>,
    pub status_detail: Option<String>,
    pub windows_job_id: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct QueueResponse {
    pub tasks: Vec<QueueTaskView>,
    pub paused: bool,
    pub printer: PrinterState,
}

pub async fn queue(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Json<QueueResponse>> {
    let rows = queue_rows(&state.pool).await?;
    let paused = settings::queue_paused(&state.pool).await?;
    Ok(Json(QueueResponse {
        tasks: rows
            .into_iter()
            .map(|row| to_view(row, &user, false))
            .collect(),
        paused,
        printer: state.printer_state.read().await.clone(),
    }))
}

pub async fn queue_rows(pool: &sqlx::SqlitePool) -> AppResult<Vec<QueueRow>> {
    Ok(sqlx::query_as::<_, QueueRow>(
        r#"
        SELECT t.id, t.user_id, u.student_id, t.file_name, t.page_count, t.odd_even,
               t.status, t.submitted_at, t.review_reason, t.status_detail, t.windows_job_id
        FROM print_tasks t
        JOIN users u ON u.id = t.user_id
        WHERE t.status IN ('queued', 'printing', 'pending_review')
        ORDER BY
            CASE t.status WHEN 'printing' THEN 0 WHEN 'queued' THEN 1 ELSE 2 END,
            t.submitted_at ASC, t.id ASC
        "#,
    )
    .fetch_all(pool)
    .await?)
}

pub fn to_view(row: QueueRow, user: &User, force_visible: bool) -> QueueTaskView {
    let mine = row.user_id == user.id;
    let visible = force_visible || user.is_admin() || mine;
    QueueTaskView {
        id: row.id,
        status: row.status,
        page_count: row.page_count,
        odd_even: row.odd_even,
        submitted_at: row.submitted_at,
        owner_name: visible.then_some(row.student_id),
        mine,
        file_name_visible: visible,
        file_name: visible.then_some(row.file_name),
        review_reason: visible.then_some(row.review_reason).flatten(),
        status_detail: visible.then_some(row.status_detail).flatten(),
        windows_job_id: visible.then_some(row.windows_job_id).flatten(),
    }
}
