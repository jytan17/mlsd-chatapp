//! Wire contract — the API shape that crosses the network.
//!
//! Domain entities, the WS protocol, and the request/response DTOs for each
//! HTTP endpoint. Single source of truth for what a frontend codes against
//! (directly if Rust, or via a JSON view if not).
//!
//! Server-internal types (sqlx row structs, extractor helpers) live in the
//! handler modules — they never cross the wire, so they stay out of here.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ===== entities =====

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub sender_id: Uuid,
    pub body: String,
}

// ===== websocket protocol =====

/// Server → client. Adjacently tagged: `{"type": "...", "data": {...}}`.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum ServerEvent {
    NewMessage(Message),
    Pong,
    Error(String),
    Typing {
        conversation_id: Uuid,
        user_id: Uuid,
    },
    Read {
        conversation_id: Uuid,
        user_id: Uuid,
        last_read_message_id: Uuid,
    },
}

/// Client → server.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum ClientEvent {
    Ping,
    SendMessage { conversation_id: Uuid, body: String },
    Typing { conversation_id: Uuid },
}

// ===== auth API =====

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SignupReq {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SignupResp {
    pub id: Uuid,
    pub username: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LoginReq {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LoginResp {
    pub token: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MeResp {
    pub id: Uuid,
    pub username: String,
}

// ===== conversations API =====

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateConvReq {
    pub kind: String,
    pub member_ids: Vec<Uuid>,
    pub name: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConvResp {
    pub id: Uuid,
    pub kind: String,
    pub member_ids: Vec<Uuid>,
    pub name: Option<String>,
}

// ===== messages API =====

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SendMsgReq {
    pub body: String,
}

#[derive(Serialize, Deserialize)]
pub struct MarkReadReq {
    pub message_id: Uuid,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ListMsgQuery {
    pub before: Option<Uuid>,
    pub after: Option<Uuid>,
    pub limit: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ListMsgResp {
    pub messages: Vec<Message>,
    pub next_cursor: Option<Uuid>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PresenceResp {
    pub user_id: Uuid,
    pub online: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UploadReq {
    pub content_type: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UploadResp {
    pub media_id: Uuid,
    pub upload_url: String,
}
