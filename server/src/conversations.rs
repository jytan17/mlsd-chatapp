use axum::{Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use crate::{AppState, auth::AuthUser};

#[derive(Deserialize)]
pub struct CreateConvReq {
    pub kind: String,
    pub member_ids: Vec<Uuid>,
    pub name: Option<String>,
}

#[derive(Serialize)]
pub struct ConvResp {
    pub id: Uuid,
    pub kind: String,
    pub member_ids: Vec<Uuid>,
    pub name: Option<String>,
}

pub async fn create_conversation(
    AuthUser(me): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<CreateConvReq>,
) -> Result<(StatusCode, Json<ConvResp>), (StatusCode, String)> {
    match req.kind.as_str() {
        "dm" => create_dm(&state, me, req).await,
        "group" => Err((
            StatusCode::NOT_IMPLEMENTED,
            "group to be implemented".into(),
        )),
        _ => Err((StatusCode::BAD_REQUEST, "kind has to be dm or group".into())),
    }
}

async fn create_dm(
    state: &AppState,
    me: Uuid,
    req: CreateConvReq,
) -> Result<(StatusCode, Json<ConvResp>), (StatusCode, String)> {
    if req.member_ids.len() != 1 {
        return Err((StatusCode::BAD_REQUEST, "dm needs exactly 1 peer".into()));
    }
    let peer = req.member_ids[0];
    if peer == me {
        return Err((StatusCode::BAD_REQUEST, "cannot dm self".into()));
    }

    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "tx begin".into()))?;

    let existing: Option<Uuid> = sqlx::query_scalar(
        r#"
        SELECT c.id
        FROM conversations c
        JOIN conversation_members m1 ON m1.conversation_id = c.id AND m1.user_id = $1
        JOIN conversation_members m2 ON m2.conversation_id = c.id AND m2.user_id = $2
        WHERE c.kind = 'dm'
        LIMIT 1
        "#,
    )
    .bind(me)
    .bind(peer)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("select {e}")))?;

    if let Some(id) = existing {
        tx.commit().await.ok();
        return Ok((
            StatusCode::OK,
            Json(ConvResp {
                id,
                kind: "dm".into(),
                member_ids: vec![me, peer],
                name: None,
            }),
        ));
    }

    let conv_id = Uuid::now_v7();
    sqlx::query("INSERT INTO conversations (id, kind) VALUES ($1, 'dm')")
        .bind(conv_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("insert conv: {e}")))?;

    sqlx::query(
        "INSERT INTO conversation_members (conversation_id, user_id) VALUES ($1, $2), ($1, $3)",
    )
    .bind(conv_id)
    .bind(me)
    .bind(peer)
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(db) = &e {
            if db.is_foreign_key_violation() {
                return (StatusCode::BAD_REQUEST, "peer user not found".into());
            }
        }
        (StatusCode::INTERNAL_SERVER_ERROR, format!("insert members: {e}"))
    })?;

    tx.commit()
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "tx commit".into()))?;

    Ok((
        StatusCode::CREATED,
        Json(ConvResp {
            id: conv_id,
            kind: "dm".into(),
            member_ids: vec![me, peer],
            name: None,
        }),
    ))
}
