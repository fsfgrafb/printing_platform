use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::{
    app::AppState,
    auth::middleware::CurrentUser,
    error::AppResult,
    services::{quota, settings},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/user/quota", get(quota_info))
        .route("/user/submit-stats", get(submit_stats))
        .route("/user/profile", post(update_profile))
        .route("/user/admin-contact", get(admin_contact))
}

#[derive(Debug, Serialize)]
pub struct QuotaResponse {
    pub used_today: i64,
    pub reserved: i64,
    pub limit: i64,
    pub remaining: i64,
}

pub async fn quota_info(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Json<QuotaResponse>> {
    let used_today = quota::used_today(&state.pool, user.id).await?;
    let limit = quota::daily_limit(&state.pool).await?;
    let reserved = quota::reserved(&state.pool, user.id).await?;
    Ok(Json(QuotaResponse {
        used_today,
        reserved,
        limit,
        remaining: (limit - used_today - reserved).max(0),
    }))
}

#[derive(Debug, Serialize)]
pub struct SubmitStatsResponse {
    pub visit_count: i64,
    pub print_total_pages: i64,
}

pub async fn submit_stats(
    State(state): State<AppState>,
    CurrentUser(_user): CurrentUser,
) -> AppResult<Json<SubmitStatsResponse>> {
    let visit_count = settings::increment_counter(&state.pool, "submit_page_visits").await?;
    let print_total_pages: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(page_count), 0) FROM print_tasks WHERE status = 'done'",
    )
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(SubmitStatsResponse {
        visit_count,
        print_total_pages,
    }))
}

#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    pub qq: Option<String>,
    pub phone: Option<String>,
}

pub async fn update_profile(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(request): Json<UpdateProfileRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let qq = optional_profile_value(request.qq);
    let phone = optional_profile_value(request.phone);
    if phone
        .as_ref()
        .is_some_and(|value| value.chars().count() > 32)
    {
        return Err(crate::error::AppError::BadRequest(
            "手机号不能超过 32 个字符".to_string(),
        ));
    }

    sqlx::query("UPDATE users SET qq = ?, phone = ? WHERE id = ?")
        .bind(qq)
        .bind(phone)
        .bind(user.id)
        .execute(&state.pool)
        .await?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

fn optional_profile_value(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[derive(Debug, Serialize)]
pub struct AdminContactResponse {
    pub student_id: String,
    pub qq: String,
}

pub async fn admin_contact(
    State(state): State<AppState>,
    CurrentUser(_user): CurrentUser,
) -> AppResult<Json<AdminContactResponse>> {
    let row = sqlx::query_as::<_, (String, Option<String>)>(
        "SELECT student_id, qq FROM users WHERE role = 'admin' ORDER BY id ASC LIMIT 1",
    )
    .fetch_optional(&state.pool)
    .await?;
    let (student_id, qq) = row.unwrap_or_else(|| (String::new(), None));
    Ok(Json(AdminContactResponse {
        student_id,
        qq: qq.unwrap_or_default(),
    }))
}
