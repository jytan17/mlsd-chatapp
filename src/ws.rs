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

use crate::{
    AppState,
    auth::AuthUser,
    contract::{ClientEvent, ServerEvent},
};

pub async fn ws_handler(
    AuthUser(user_id): AuthUser,
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, user_id, state))
}

async fn handle_socket(socket: WebSocket, user_id: Uuid, state: AppState) {
    let (mut sink, mut stream) = socket.split();
    let (tx, mut rx) = mpsc::channel::<String>(32);

    state
        .hub
        .lock()
        .unwrap()
        .entry(user_id)
        .or_default()
        .push(tx.clone());

    let mut hb_redis = state.redis.clone();
    let hb_task = tokio::spawn(async move {
        let key = format!("presence:user:{user_id}");
        loop {
            let _ = redis::cmd("SET")
                .arg(&key)
                .arg("1")
                .arg("EX")
                .arg(20)
                .exec_async(&mut hb_redis)
                .await;
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        }
    });

    let conv_ids: Vec<Uuid> =
        sqlx::query_scalar("SELECT conversation_id FROM conversation_members WHERE user_id = $1")
            .bind(user_id)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();

    let psink = state.pubsub_sink.clone();
    for c in &conv_ids {
        crate::fanout::add_sub(&psink, &state.subs, *c).await;
    }

    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sink.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    let ping_tx = tx.clone();
    let db = state.db.clone();
    let mut msg_redis = state.redis.clone();
    let recv_task = async move {
        while let Some(Ok(msg)) = stream.next().await {
            match msg {
                Message::Text(txt) => match serde_json::from_str::<ClientEvent>(&txt) {
                    Ok(ClientEvent::Ping) => {
                        let pong = serde_json::to_string(&ServerEvent::Pong).unwrap();
                        let _ = ping_tx.send(pong);
                    }
                    Ok(ClientEvent::SendMessage {
                        conversation_id,
                        body,
                    }) => {
                        if let Err((_, e)) = crate::messages::create_message(
                            &db,
                            &mut msg_redis,
                            user_id,
                            conversation_id,
                            body,
                        )
                        .await
                        {
                            let err =
                                serde_json::to_string(&ServerEvent::Error(e.to_string())).unwrap();
                            let _ = ping_tx.send(err);
                        }
                    }
                    Ok(ClientEvent::Typing { conversation_id }) => {
                        crate::messages::broadcast_typing(
                            &db,
                            &mut msg_redis,
                            user_id,
                            conversation_id,
                        )
                        .await;
                    }
                    Err(_) => {}
                },
                _ => {}
            }
        }
    };

    tokio::select! {
        _ = &mut send_task => {},
        _ = recv_task => send_task.abort(),
    }
    hb_task.abort();

    let current: Vec<Uuid> =
        sqlx::query_scalar("SELECT conversation_id FROM conversation_members WHERE user_id = $1")
            .bind(user_id)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();

    for c in &current {
        crate::fanout::remove_sub(&psink, &state.subs, *c).await;
    }

    let mut guard = state.hub.lock().unwrap();
    if let Some(v) = guard.get_mut(&user_id) {
        v.retain(|s| !s.same_channel(&tx));
        if v.is_empty() {
            guard.remove(&user_id);
        }
    }
}
