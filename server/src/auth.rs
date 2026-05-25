use axum::{
    Json,
    extract::{FromRequestParts, State},
    http::{StatusCode, request::Parts},
};
use redis::AsyncCommands;
use serde::Serialize;
use uuid::Uuid;

use crate::AppState;

pub struct AuthUser(pub Uuid);

#[derive(Serialize)]
pub struct MeResp {
    pub id: Uuid,
    pub username: String,
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or((StatusCode::UNAUTHORIZED, "missing auth"))?;

        let token = header
            .strip_prefix("Bearer ")
            .ok_or((StatusCode::UNAUTHORIZED, "bad auth format"))?;

        let mut redis = state.redis.clone();
        let key = format!("session:{token}");
        let user_id_str: Option<String> = redis
            .get(&key)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "redis"))?;

        let user_id_str = user_id_str.ok_or((StatusCode::UNAUTHORIZED, "invalid token"))?;

        let user_id = Uuid::parse_str(&user_id_str)
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "corrupt session"))?;

        Ok(AuthUser(user_id))
    }
}

pub async fn me(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<MeResp>, (StatusCode, &'static str)> {
    let row: (String,) = sqlx::query_as("SELECT username FROM users WHERE id = $1")
        .bind(auth.0)
        .fetch_one(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db"))?;

    Ok(Json(MeResp {
        id: auth.0,
        username: row.0,
    }))
}
