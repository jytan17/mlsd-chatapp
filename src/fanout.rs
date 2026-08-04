use crate::Hub;
use futures_util::StreamExt;
use redis::aio::{PubSubSink, PubSubStream};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

pub type Subs = Arc<Mutex<HashMap<Uuid, usize>>>;

pub fn channel(conv_id: Uuid) -> String {
    format!("conv:{conv_id}")
}

#[derive(Serialize, Deserialize)]
pub struct Broadcast {
    pub members: Vec<Uuid>,
    pub payload: String,
}

pub async fn add_sub(sink: &mut PubSubSink, subs: &Subs, conv_id: Uuid) {
    let first = {
        let mut g = subs.lock().unwrap();
        let c = g.entry(conv_id).or_insert(0);
        *c += 1;
        *c == 1
    };
    if first {
        let _ = sink.subscribe(channel(conv_id)).await;
    }
}

pub async fn remove_sub(sink: &mut PubSubSink, subs: &Subs, conv_id: Uuid) {
    let last = {
        let mut g = subs.lock().unwrap();
        match g.get_mut(&conv_id) {
            Some(c) => {
                *c -= 1;
                if *c == 0 {
                    g.remove(&conv_id);
                    true
                } else {
                    false
                }
            }
            None => false,
        }
    };
    if last {
        let _ = sink.unsubscribe(channel(conv_id)).await;
    }
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

pub async fn run_subscriber(mut stream: PubSubStream, hub: Hub) {
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
