pub mod admin;
pub mod print;
pub mod queue;
pub mod user;

use axum::{
    extract::{DefaultBodyLimit, State},
    response::Html,
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use tower_http::{
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};

use crate::{
    app::AppState,
    auth,
    error::{AppError, AppResult},
    services::{printer::PrinterState, settings},
    ws,
};

pub fn router(state: AppState) -> Router {
    let api = Router::new()
        .route("/auth/login", post(auth::login::login))
        .route("/auth/logout", post(auth::login::logout))
        .route("/auth/me", get(auth::login::me))
        .route("/auth/change-password", post(auth::login::change_password))
        .route("/ws/queue", get(ws::queue_ws))
        .merge(health_router())
        .merge(user::router())
        .merge(print::router())
        .merge(queue::router())
        .merge(admin::router())
        .fallback(api_not_found)
        .layer(DefaultBodyLimit::disable())
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    Router::new()
        .nest("/api", api)
        .nest_service("/assets", ServeDir::new("frontend/dist/assets"))
        .route_service("/favicon.svg", ServeFile::new("frontend/dist/favicon.svg"))
        .route("/", get(spa_index))
        .fallback(spa_index)
        .with_state(state)
}

async fn spa_index() -> AppResult<Html<String>> {
    let html = tokio::fs::read_to_string("frontend/dist/index.html")
        .await
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                AppError::NotFound(
                    "frontend/dist/index.html not found; run `cd frontend && npm run build`"
                        .to_string(),
                )
            } else {
                error.into()
            }
        })?;
    Ok(Html(html))
}

async fn api_not_found() -> AppError {
    AppError::NotFound("api endpoint not found".to_string())
}

fn health_router() -> Router<AppState> {
    Router::new().route("/health", get(health))
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    ok: bool,
    version: &'static str,
    database: &'static str,
    queue_paused: bool,
    printer: PrinterState,
}

async fn health(State(state): State<AppState>) -> AppResult<Json<HealthResponse>> {
    sqlx::query_scalar::<_, i64>("SELECT 1")
        .fetch_one(&state.pool)
        .await?;

    Ok(Json(HealthResponse {
        ok: true,
        version: env!("CARGO_PKG_VERSION"),
        database: "ok",
        queue_paused: settings::queue_paused(&state.pool).await?,
        printer: state.printer_state.read().await.clone(),
    }))
}
