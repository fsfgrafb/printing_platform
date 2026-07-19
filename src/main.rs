mod app;
mod auth;
mod config;
mod db;
mod error;
mod routes;
mod services;
mod utils;
mod ws;

use app::AppState;
use config::Config;
use error::AppResult;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> AppResult<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "printing_platform=info,tower_http=info,axum=info".into()),
        )
        .with(tracing_subscriber::fmt::layer().with_ansi(false))
        .init();

    let config = Config::load()?;
    utils::ensure_data_dirs(&config).await?;

    let pool = db::connect(&config.database_url()).await?;
    db::migrate::run(&pool).await?;
    services::settings::initialize_daily_limit(&pool, config.default_daily_limit).await?;
    auth::login::ensure_initial_admin(&pool, &config).await?;

    let state = AppState::new(pool, config);
    services::queue_manager::spawn(state.clone());
    services::cleanup::spawn(state.clone());

    let app = routes::router(state.clone());
    let listener = TcpListener::bind(&state.config.server.bind).await?;
    info!(
        bind = %state.config.server.bind,
        data_dir = %state.config.data_dir.display(),
        simulate = state.config.printer.simulate,
        "printing platform started"
    );

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(windows)]
    let terminate = async {
        tokio::signal::windows::ctrl_close()
            .expect("failed to install Ctrl+Close handler")
            .recv()
            .await;
    };

    #[cfg(not(windows))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
