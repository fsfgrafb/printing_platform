use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;

use crate::{
    app::AppState,
    error::AppResult,
    services::{printer, settings},
};

pub fn router() -> Router<AppState> {
    Router::new().route("/health", get(health))
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub ok: bool,
    pub version: &'static str,
    pub database: &'static str,
    pub queue_paused: bool,
    pub printer_status: String,
}

pub async fn health(State(state): State<AppState>) -> AppResult<Json<HealthResponse>> {
    sqlx::query_scalar::<_, i64>("SELECT 1")
        .fetch_one(&state.pool)
        .await?;

    Ok(Json(HealthResponse {
        ok: true,
        version: env!("CARGO_PKG_VERSION"),
        database: "ok",
        queue_paused: settings::queue_paused(&state.pool).await?,
        printer_status: printer::status(&state.config).await,
    }))
}
