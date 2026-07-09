pub mod handler;

use serde::Serialize;
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct Broadcaster {
    sender: broadcast::Sender<QueueEvent>,
}

impl Broadcaster {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<QueueEvent> {
        self.sender.subscribe()
    }

    pub fn send(&self, event: QueueEvent) {
        let _ = self.sender.send(event);
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum QueueEvent {
    QueueChanged,
    TaskStatus { task_id: i64, status: String },
    QueuePaused { paused: bool },
    PrinterError { message: String },
}
