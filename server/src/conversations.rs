use axum::{Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{AppState, auth::AuthUser};

#[derive(Deserialize)]
pub struct CreateConvReq {
    pub kind: String,
    pub member_ids: Vec<Uuid>,
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Serialize)]
pub struct CreateConvResp {
    pub id: Uuid,
    pub kind: String,
}

#[derive(Serialize)]
pub struct ConvSummary {
    pub id: Uuid,
    pub kind: String,
    pub name: Option<String>,
    pub member_ids: Vec<Uuid>,
}

pub async fn create_conversation(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(req): Json<CreateConvReq>,
) -> Result<(StatusCode, Json<CreateConvResp>), (StatusCode, String)> {
    let me = auth.0;

    match req.kind.as_str() {
        "dm" => create_dm(state, me, req.member_ids).await,
        "group" => Err((StatusCode::NOT_IMPLEMENTED, "group later".into())),
        _ => Err((StatusCode::BAD_REQUEST, "invalid kind".into())),
    }
}

async fn create_dm(
    state: AppState,
    me: Uuid,
    member_ids: Vec<Uuid>,
) -> Result<(StatusCode, Json<CreateConvResp>), (StatusCode, String)> {
    if member_ids.len() != 1 {
        return Err((StatusCode::BAD_REQUEST, "dm needs exactly 1 peer".into()));
    }

    let peer = member_ids[0];
    if peer == me {
        return Err((StatusCode::BAD_REQUEST, "cannot dm self".into()));
    }

    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "tx begin".into()))?;

    let existing: Option<(Uuid,)> = sqlx::query_as(
        r#"
        SELECT c.id FROM conversations c
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
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db".into()))?;

    if let Some((id,)) = existing {
        tx.commit().await.ok();
        return Ok((
            StatusCode::OK,
            Json(CreateConvResp {
                id,
                kind: "dm".into(),
            }),
        ));
    }

    let conv_id = Uuid::now_v7();
    sqlx::query("INSERT INTO conversations (id, kind) VALUES ($1, 'dm')")
        .bind(conv_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "insert conv".into()))?;

    sqlx::query(
        "INSERT INTO conversation_members (conversation_id, user_id) VALUES ($1, $2), ($1, $3)",
    )
    .bind(conv_id)
    .bind(me)
    .bind(peer)
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        let msg = if matches!(&e, sqlx::Error::Database(db) if db.is_foreign_key_violation()) {
            "peer not found"
        } else {
            "insert members"
        };
        (StatusCode::BAD_REQUEST, msg.into())
    })?;

    tx.commit()
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "tx commit".into()))?;

    Ok((
        StatusCode::CREATED,
        Json(CreateConvResp {
            id: conv_id,
            kind: "dm".into(),
        }),
    ))
}

pub async fn list_conversations(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<ConvSummary>>, (StatusCode, String)> {
    let rows: Vec<(Uuid, String, Option<String>, Vec<Uuid>)> = sqlx::query_as(
        r#"
        SELECT c.id, c.kind, c.name, array_agg(cm2.user_id) AS member_ids
        FROM conversations c
        JOIN conversation_members cm  ON cm.conversation_id  = c.id AND cm.user_id = $1
        JOIN conversation_members cm2 ON cm2.conversation_id = c.id
        GROUP BY c.id
        ORDER BY c.id DESC
        LIMIT 100
        "#,
    )
    .bind(auth.0)
    .fetch_all(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db".into()))?;

    let out = rows
        .into_iter()
        .map(|(id, kind, name, member_ids)| ConvSummary {
            id,
            kind,
            name,
            member_ids,
        })
        .collect();

    Ok(Json(out))
}
