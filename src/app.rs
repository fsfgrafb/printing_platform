use std::sync::Arc;

use sqlx::SqlitePool;

use crate::{config::Config, ws::Broadcaster};

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub config: Arc<Config>,
    pub broadcaster: Broadcaster,
}

impl AppState {
    pub fn new(pool: SqlitePool, config: Config) -> Self {
        Self {
            pool,
            config: Arc::new(config),
            broadcaster: Broadcaster::new(128),
        }
    }
}
