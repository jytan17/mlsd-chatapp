use axum::{
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::{AppState, Hub, auth::AuthUser};

pub async fn ws_handler(
    AuthUser(user_id): AuthUser,
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, user_id, state.hub))
}

async fn handle_socket(socket: WebSocket, user_id: Uuid, hub: Hub) {
    let (mut sink, mut stream) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    hub.lock()
        .unwrap()
        .entry(user_id)
        .or_default()
        .push(tx.clone());

    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sink.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    let recv_task = async {
        while let Some(Ok(msg)) = stream.next().await {
            if let Message::Close(_) = msg {
                break;
            }
        }
    };

    tokio::select! {
        _ = &mut send_task => {},
        _ = recv_task => send_task.abort(),
    }

    let mut guard = hub.lock().unwrap();
    if let Some(v) = guard.get_mut(&user_id) {
        v.retain(|s| !s.same_channel(&tx));
        if v.is_empty() {
            guard.remove(&user_id);
        }
    }
}
