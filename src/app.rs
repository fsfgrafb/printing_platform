use std::sync::Arc;

use sqlx::SqlitePool;
use tokio::sync::{Mutex, RwLock, Semaphore};

use crate::{config::Config, services::printer::PrinterState, ws::Broadcaster};

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub config: Arc<Config>,
    pub broadcaster: Broadcaster,
    pub printer_state: Arc<RwLock<PrinterState>>,
    pub submission_lock: Arc<Mutex<()>>,
    pub queue_lock: Arc<Mutex<()>>,
    pub conversion_slots: Arc<Semaphore>,
}

impl AppState {
    pub fn new(pool: SqlitePool, config: Config) -> Self {
        let conversion_concurrency = config.limits.conversion_concurrency;
        Self {
            pool,
            config: Arc::new(config),
            broadcaster: Broadcaster::new(128),
            printer_state: Arc::new(RwLock::new(PrinterState::starting())),
            submission_lock: Arc::new(Mutex::new(())),
            queue_lock: Arc::new(Mutex::new(())),
            conversion_slots: Arc::new(Semaphore::new(conversion_concurrency)),
        }
    }
}
