use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::{Json, extract::State, http::StatusCode};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::random;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AppState;

const SESSION_TTL_SECS: u64 = 60 * 60 * 24 * 30; // 30 days
const DUMMY_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$ZmFrZXNhbHRmYWtlc2FsdA$dGFFu/X/oGz4FZJxFwQ7gKQ7g7vYfGqQpFqz5gKlF8Y";

#[derive(Deserialize)]
pub struct LoginReq {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResp {
    pub token: String,
}

pub async fn login(
    State(mut state): State<AppState>,
    Json(req): Json<LoginReq>,
) -> Result<Json<LoginResp>, (StatusCode, &'static str)> {
    let row: Option<(Uuid, String)> =
        sqlx::query_as("SELECT id, password_hash FROM users WHERE username = $1")
            .bind(&req.username)
            .fetch_optional(&state.db)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db"))?;

    let (user_id, stored_hash) = match row {
        Some((id, h)) => (Some(id), h),
        None => (None, DUMMY_HASH.to_string()),
    };

    let parsed = PasswordHash::new(&stored_hash)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "bad stored hash"))?;

    let verify_ok = Argon2::default()
        .verify_password(req.password.as_bytes(), &parsed)
        .is_ok();

    let user_id = match (user_id, verify_ok) {
        (Some(id), true) => id,
        _ => return Err((StatusCode::UNAUTHORIZED, "invalid credentials")),
    };

    let bytes: [u8; 32] = random();
    let token = URL_SAFE_NO_PAD.encode(bytes);

    let key = format!("session:{token}");
    let _: () = state
        .redis
        .set_ex(&key, user_id.to_string(), SESSION_TTL_SECS)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "redis"))?;

    Ok(Json(LoginResp { token }))
}
