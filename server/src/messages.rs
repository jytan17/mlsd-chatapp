use axum::{
    Json,
    extract::Query,
    extract::{Path, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{AppState, auth::AuthUser};

#[derive(Deserialize)]
pub struct SendMsgReq {
    pub body: String,
}

#[derive(Serialize)]
pub struct MsgResp {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub sender_id: Uuid,
    pub body: String,
}

#[derive(Deserialize)]
pub struct ListMsgQuery {
    pub before: Option<Uuid>,
    pub limit: Option<i64>,
}

#[derive(Serialize)]
pub struct ListMsgResp {
    pub messages: Vec<MsgResp>,
    pub next_cursor: Option<Uuid>,
}

pub async fn send_message(
    AuthUser(me): AuthUser,
    State(state): State<AppState>,
    Path(conv_id): Path<Uuid>,
    Json(req): Json<SendMsgReq>,
) -> Result<(StatusCode, Json<MsgResp>), (StatusCode, &'static str)> {
    if req.body.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "empty body"));
    }
    if req.body.len() > 4096 {
        return Err((StatusCode::BAD_REQUEST, "body too long"));
    }

    let is_member: Option<i32> = sqlx::query_scalar(
        "SELECT 1 FROM conversation_members WHERE conversation_id = $1 AND user_id = $2",
    )
    .bind(conv_id)
    .bind(me)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db err"))?;

    if is_member.is_none() {
        return Err((StatusCode::FORBIDDEN, "not a member"));
    }

    let msg_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO messages (id, conversation_id, sender_id, body) VALUES ($1, $2, $3, $4)",
    )
    .bind(msg_id)
    .bind(conv_id)
    .bind(me)
    .bind(&req.body)
    .execute(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "insert err"))?;

    Ok((
        StatusCode::CREATED,
        Json(MsgResp {
            id: msg_id,
            conversation_id: conv_id,
            sender_id: me,
            body: req.body,
        }),
    ))
}

pub async fn list_messages(
    AuthUser(me): AuthUser,
    State(state): State<AppState>,
    Path(conv_id): Path<Uuid>,
    Query(q): Query<ListMsgQuery>,
) -> Result<Json<ListMsgResp>, (StatusCode, &'static str)> {
    let limit = q.limit.unwrap_or(50).clamp(1, 100);

    let is_member: Option<i32> = sqlx::query_scalar(
        "SELECT 1 FROM conversation_members WHERE conversation_id = $1 AND user_id = $2",
    )
    .bind(conv_id)
    .bind(me)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db err"))?;

    if is_member.is_none() {
        return Err((StatusCode::FORBIDDEN, "not a member"));
    }

    let rows: Vec<(Uuid, Uuid, Uuid, String)> = match q.before {
        Some(before) => sqlx::query_as(
            "SELECT
                id, conversation_id, sender_id, body
            FROM messages
            WHERE conversation_id = $1 AND id < $2
            ORDER BY id DESC
            LIMIT $3
            ",
        )
        .bind(conv_id)
        .bind(before)
        .bind(limit),
        None => sqlx::query_as(
            "
            SELECT
                id, conversation_id, sender_id, body
            FROM messages
            WHERE conversation_id = $1
            ORDER BY id DESC
            LIMIT $2
            ",
        )
        .bind(conv_id)
        .bind(limit),
    }
    .fetch_all(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db err"))?;

    let next_cursor = if rows.len() as i64 == limit {
        rows.last().map(|r| r.0)
    } else {
        None
    };

    let messages = rows
        .into_iter()
        .map(|(id, conversation_id, sender_id, body)| MsgResp {
            id,
            conversation_id,
            sender_id,
            body,
        })
        .collect();

    Ok(Json(ListMsgResp {
        messages,
        next_cursor,
    }))
}
