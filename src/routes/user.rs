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
        .route("/user/profile", post(update_profile))
        .route("/user/admin-contact", get(admin_contact))
}

#[derive(Debug, Serialize)]
pub struct QuotaResponse {
    pub used_today: i64,
    pub limit: i64,
    pub remaining: i64,
}

pub async fn quota_info(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Json<QuotaResponse>> {
    let used_today = quota::used_today(&state.pool, user.id).await?;
    let limit = quota::daily_limit(&state.pool).await?;
    Ok(Json(QuotaResponse {
        used_today,
        limit,
        remaining: (limit - used_today).max(0),
    }))
}

#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    pub qq: Option<String>,
}

pub async fn update_profile(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(request): Json<UpdateProfileRequest>,
) -> AppResult<Json<serde_json::Value>> {
    sqlx::query("UPDATE users SET qq = ? WHERE id = ?")
        .bind(request.qq.as_deref().map(str::trim))
        .bind(user.id)
        .execute(&state.pool)
        .await?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Debug, Serialize)]
pub struct AdminContactResponse {
    pub student_id: String,
    pub qq: String,
}

pub async fn admin_contact(State(state): State<AppState>) -> AppResult<Json<AdminContactResponse>> {
    let student_id = settings::get_or(&state.pool, "admin_student_id", "").await?;
    let qq = settings::get_or(&state.pool, "admin_qq", "").await?;
    Ok(Json(AdminContactResponse { student_id, qq }))
}
