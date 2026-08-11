use crate::contract::{ListMsgQuery, ListMsgResp, MarkReadReq, Message, SendMsgReq, ServerEvent};
use crate::fanout::{Broadcast, channel};
use crate::{AppState, auth::AuthUser};
use axum::{
    Json,
    extract::Query,
    extract::{Path, State},
    http::StatusCode,
};
use redis::aio::ConnectionManager;
use sqlx::PgPool;
use uuid::Uuid;

pub async fn send_message(
    AuthUser(me): AuthUser,
    State(state): State<AppState>,
    Path(conv_id): Path<Uuid>,
    Json(req): Json<SendMsgReq>,
) -> Result<(StatusCode, Json<Message>), (StatusCode, &'static str)> {
    let mut redis = state.redis.clone();
    let msg = create_message(&state.db, &mut redis, me, conv_id, req.body).await?;
    Ok((StatusCode::CREATED, Json(msg)))
}

pub async fn create_message(
    db: &PgPool,
    redis: &mut ConnectionManager,
    sender_id: Uuid,
    conv_id: Uuid,
    body: String,
) -> Result<Message, (StatusCode, &'static str)> {
    if body.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "empty body"));
    }
    if body.len() > 4096 {
        return Err((StatusCode::BAD_REQUEST, "body too long"));
    }

    // rate limit user level
    let rl_key = format!("rate:msg:{sender_id}");
    let n: i64 = redis::cmd("INCR")
        .arg(&rl_key)
        .query_async(&mut *redis)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "redis err"))?;

    if n == 1 {
        let _: () = redis::cmd("EXPIRE")
            .arg(&rl_key)
            .arg(10)
            .query_async(&mut *redis)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "redis err"))?;
    }

    if n > 30 {
        return Err((StatusCode::TOO_MANY_REQUESTS, "rate limited"));
    }

    let is_member: Option<i32> = sqlx::query_scalar(
        "SELECT 1 FROM conversation_members WHERE conversation_id = $1 AND user_id = $2",
    )
    .bind(conv_id)
    .bind(sender_id)
    .fetch_optional(db)
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
    .bind(sender_id)
    .bind(&body)
    .execute(db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "insert err"))?;

    let members: Vec<Uuid> =
        sqlx::query_scalar("SELECT user_id FROM conversation_members WHERE conversation_id = $1")
            .bind(conv_id)
            .fetch_all(db)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db err"))?;

    let msg = Message {
        id: msg_id,
        conversation_id: conv_id,
        sender_id: sender_id,
        body: body,
    };

    let event = ServerEvent::NewMessage(msg.clone());
    let payload = serde_json::to_string(&event).unwrap();
    let broadcast = Broadcast { members, payload };
    let raw = serde_json::to_string(&broadcast).unwrap();
    let mut conn = redis.clone();
    for attempt in 0..3 {
        match redis::cmd("PUBLISH")
            .arg(channel(conv_id))
            .arg(&raw)
            .exec_async(&mut conn)
            .await
        {
            Ok(()) => break,
            Err(e) => {
                eprintln!("publish attempt {attempt} failed: {e}");
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }
    }
    Ok(msg)
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

    let rows: Vec<(Uuid, Uuid, Uuid, String)> = match (q.after, q.before) {
        (Some(after), _) => sqlx::query_as(
            "SELECT
                id, conversation_id, sender_id, body
            FROM messages
            WHERE conversation_id = $1 AND id > $2
            ORDER BY id ASC
            LIMIT $3
            ",
        )
        .bind(conv_id)
        .bind(after)
        .bind(limit),

        (_, Some(before)) => sqlx::query_as(
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
        (None, None) => sqlx::query_as(
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
        .map(|(id, conversation_id, sender_id, body)| Message {
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

pub async fn mark_read(
    AuthUser(me): AuthUser,
    State(state): State<AppState>,
    Path(conv_id): Path<Uuid>,
    Json(req): Json<MarkReadReq>,
) -> Result<StatusCode, (StatusCode, &'static str)> {
    let res = sqlx::query(
        "UPDATE conversation_members
        SET last_read_message_id = $1
        WHERE conversation_id = $2 AND user_id = $3
            AND (last_read_message_id IS NULL OR $1 > last_read_message_id)",
    )
    .bind(req.message_id)
    .bind(conv_id)
    .bind(me)
    .execute(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db err"))?;

    if res.rows_affected() == 0 {
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
        return Ok(StatusCode::NO_CONTENT);
    }

    let members: Vec<Uuid> =
        sqlx::query_scalar("SELECT user_id FROM conversation_members WHERE conversation_id = $1")
            .bind(conv_id)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();

    let targets: Vec<Uuid> = members.into_iter().filter(|m| *m != me).collect();
    if !targets.is_empty() {
        let event = ServerEvent::Read {
            conversation_id: conv_id,
            user_id: me,
            last_read_message_id: req.message_id,
        };
        let payload = serde_json::to_string(&event).unwrap();
        let bc = Broadcast {
            members: targets,
            payload,
        };
        let raw = serde_json::to_string(&bc).unwrap();
        let mut conn = state.redis.clone();
        let _ = redis::cmd("PUBLISH")
            .arg(channel(conv_id))
            .arg(&raw)
            .exec_async(&mut conn)
            .await;
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn broadcast_typing(
    db: &PgPool,
    redis: &mut ConnectionManager,
    sender_id: Uuid,
    conv_id: Uuid,
) {
    let members: Vec<Uuid> =
        sqlx::query_scalar("SELECT user_id FROM conversation_members WHERE conversation_id = $1")
            .bind(conv_id)
            .fetch_all(db)
            .await
            .unwrap_or_default();

    if !members.contains(&sender_id) {
        return;
    }
    let targets: Vec<Uuid> = members.into_iter().filter(|m| *m != sender_id).collect();
    if targets.is_empty() {
        return;
    }

    let event = ServerEvent::Typing {
        conversation_id: conv_id,
        user_id: sender_id,
    };
    let payload = serde_json::to_string(&event).unwrap();
    let bc = Broadcast {
        members: targets,
        payload,
    };
    let raw = serde_json::to_string(&bc).unwrap();
    let _ = redis::cmd("PUBLISH")
        .arg(channel(conv_id))
        .arg(&raw)
        .exec_async(redis)
        .await;
}
