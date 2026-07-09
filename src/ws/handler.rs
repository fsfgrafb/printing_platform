use axum::extract::ws::{Message, WebSocket};
use axum::{
    extract::{State, WebSocketUpgrade},
    response::IntoResponse,
};
use tokio::sync::broadcast;

use crate::{app::AppState, auth::middleware::CurrentUser, ws::QueueEvent};

pub async fn queue_ws(
    ws: WebSocketUpgrade,
    CurrentUser(_user): CurrentUser,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let receiver = state.broadcaster.subscribe();
    ws.on_upgrade(move |socket| handle_socket(socket, receiver))
}

async fn handle_socket(mut socket: WebSocket, mut receiver: broadcast::Receiver<QueueEvent>) {
    while let Ok(event) = receiver.recv().await {
        let Ok(payload) = serde_json::to_string(&event) else {
            continue;
        };
        if socket.send(Message::Text(payload)).await.is_err() {
            break;
        }
    }
}
