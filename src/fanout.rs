use crate::Hub;
use futures_util::StreamExt;
use redis::aio::{PubSubSink, PubSubStream};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use uuid::Uuid;

pub type Subs = Arc<Mutex<HashMap<Uuid, usize>>>;
pub type SharedSink = Arc<tokio::sync::Mutex<PubSubSink>>;

pub fn channel(conv_id: Uuid) -> String {
    format!("conv:{conv_id}")
}

#[derive(Serialize, Deserialize)]
pub struct Broadcast {
    pub members: Vec<Uuid>,
    pub payload: String,
}

pub async fn add_sub(sink: &SharedSink, subs: &Subs, conv_id: Uuid) {
    let first = {
        let mut g = subs.lock().unwrap();
        let c = g.entry(conv_id).or_insert(0);
        *c += 1;
        *c == 1
    };
    if first {
        let _ = sink.lock().await.subscribe(channel(conv_id)).await;
    }
}

pub async fn remove_sub(sink: &SharedSink, subs: &Subs, conv_id: Uuid) {
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
        let _ = sink.lock().await.unsubscribe(channel(conv_id)).await;
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

pub async fn run_subscriber(
    client: redis::Client,
    shared_sink: SharedSink,
    subs: Subs,
    hub: Hub,
    first_stream: PubSubStream,
) {
    let mut stream = first_stream;
    loop {
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

        eprintln!("pubsub stream dropped reconnecting");
        stream = loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
            match client.get_async_pubsub().await {
                Ok(pubsub) => {
                    let (sink, new_stream) = pubsub.split();
                    let mut guard = shared_sink.lock().await;
                    *guard = sink;

                    let chans: Vec<Uuid> = subs.lock().unwrap().keys().copied().collect();
                    for c in &chans {
                        let _ = guard.subscribe(channel(*c)).await;
                    }
                    eprintln!("pubsub reconnected, resubscribed {} chans", chans.len());
                    break new_stream;
                }
                Err(_) => continue,
            }
        };
    }
}
