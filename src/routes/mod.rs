pub mod admin;
pub mod health;
pub mod history;
pub mod print;
pub mod queue;
pub mod user;

use axum::{
    extract::DefaultBodyLimit,
    response::Html,
    routing::{get, post},
    Router,
};
use tower_http::{cors::CorsLayer, services::ServeDir, trace::TraceLayer};

use crate::{
    app::AppState,
    auth,
    error::{AppError, AppResult},
    ws,
};

pub fn router(state: AppState) -> Router {
    let api = Router::new()
        .route("/auth/login", post(auth::login::login))
        .route("/auth/logout", post(auth::login::logout))
        .route("/auth/me", get(auth::login::me))
        .route("/auth/change-password", post(auth::login::change_password))
        .route("/ws/queue", get(ws::queue_ws))
        .merge(health::router())
        .merge(user::router())
        .merge(history::router())
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
