use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::IntoResponse,
};
use serde::Serialize;
use tokio::sync::{broadcast, broadcast::Receiver};
use tokio::time::{interval, Duration};

use crate::{app::AppState, auth::middleware::CurrentUser, services::printer::PrinterState};

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
    PrinterStatus { printer: PrinterState },
}

pub async fn queue_ws(
    ws: WebSocketUpgrade,
    CurrentUser(_user): CurrentUser,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let receiver = state.broadcaster.subscribe();
    ws.on_upgrade(move |socket| handle_socket(socket, receiver))
}

async fn handle_socket(mut socket: WebSocket, mut receiver: Receiver<QueueEvent>) {
    let mut heartbeat = interval(Duration::from_secs(30));
    loop {
        tokio::select! {
            event = receiver.recv() => {
                let Ok(event) = event else { break };
                let Ok(payload) = serde_json::to_string(&event) else { continue };
                if socket.send(Message::Text(payload)).await.is_err() { break; }
            }
            incoming = socket.recv() => {
                match incoming {
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                    Some(Ok(Message::Ping(payload))) => {
                        if socket.send(Message::Pong(payload)).await.is_err() { break; }
                    }
                    _ => {}
                }
            }
            _ = heartbeat.tick() => {
                if socket.send(Message::Ping(Vec::new())).await.is_err() { break; }
            }
        }
    }
}
