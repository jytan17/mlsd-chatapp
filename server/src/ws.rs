use axum::{
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use uuid::Uuid;

use crate::{AppState, auth::AuthUser};

pub async fn ws_handler(
    AuthUser(user_id): AuthUser,
    State(_state): State<AppState>,
    ws: WebSocketUpgrade,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, user_id))
}

async fn handle_socket(mut socket: WebSocket, user_id: Uuid) {
    let hello = format!("hello {user_id}");
    let _ = socket.send(Message::Text(hello.into())).await;

    while let Some(Ok(msg)) = socket.next().await {
        match msg {
            Message::Text(t) => {
                let echo = format!("echo: {t}");
                if socket.send(Message::Text(echo.into())).await.is_err() {
                    break;
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
}
