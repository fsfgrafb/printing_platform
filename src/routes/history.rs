use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::{app::AppState, auth::middleware::CurrentUser, error::AppResult};

pub fn router() -> Router<AppState> {
    Router::new().route("/user/history", get(user_history))
}

#[derive(Debug, Deserialize)]
pub struct PageQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct HistoryItem {
    pub id: i64,
    pub file_name: String,
    pub page_count: i64,
    pub odd_even: String,
    pub status: String,
    pub submitted_at: String,
    pub completed_at: Option<String>,
    pub review_reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct HistoryResponse {
    pub items: Vec<HistoryItem>,
    pub page: i64,
    pub per_page: i64,
    pub total: i64,
}

pub async fn user_history(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Query(query): Query<PageQuery>,
) -> AppResult<Json<HistoryResponse>> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * per_page;

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM print_tasks WHERE user_id = ?")
        .bind(user.id)
        .fetch_one(&state.pool)
        .await?;

    let items = sqlx::query_as::<_, HistoryItem>(
        r#"
        SELECT id, file_name, page_count, odd_even, status, submitted_at, completed_at, review_reason
        FROM print_tasks
        WHERE user_id = ?
        ORDER BY id DESC
        LIMIT ? OFFSET ?
        "#,
    )
    .bind(user.id)
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(HistoryResponse {
        items,
        page,
        per_page,
        total,
    }))
}
