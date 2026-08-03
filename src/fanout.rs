use crate::Hub;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const CHANNEL: &str = "message";

#[derive(Serialize, Deserialize)]
pub struct Broadcast {
    pub members: Vec<Uuid>,
    pub payload: String,
}

pub fn local_fanout(hub: &Hub, members: &[Uuid], payload: &str) {
    let hub = hub.lock().unwrap();
    for m in members {
        if let Some(senders) = hub.get(m) {
            for s in senders {
                let _ = s.send(payload.to_string());
            }
        }
    }
}

pub async fn run_subscriber(client: redis::Client, hub: Hub) {
    let mut pubsub = client.get_async_pubsub().await.expect("pubsub connect");
    pubsub.subscribe(CHANNEL).await.expect("subscibe");
    let mut stream = pubsub.on_message();
    while let Some(msg) = stream.next().await {
        let raw: String = match msg.get_payload() {
            Ok(p) => p,
            Err(_) => continue,
        };
        let bc: Broadcast = match serde_json::from_str(&raw) {
            Ok(b) => b,
            Err(_) => continue,
        };
        local_fanout(&hub, &bc.members, &bc.payload);
    }
}
