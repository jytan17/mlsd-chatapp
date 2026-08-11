use crate::contract::{LoginReq, LoginResp};
use argon2::{
    Argon2, PasswordHash, PasswordVerifier,
    password_hash::rand_core::{OsRng, RngCore},
};
use axum::{
    Json,
    extract::{ConnectInfo, State},
    http::StatusCode,
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use std::net::SocketAddr;
use uuid::Uuid;

use crate::AppState;

const DUMMY_HASH: &str = "tatftahbbktfatene";
const SESSION_TTL_SECS: u64 = 60 * 60 * 24 * 30;

pub async fn login(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(mut state): State<AppState>,
    Json(req): Json<LoginReq>,
) -> Result<Json<LoginResp>, (StatusCode, &'static str)> {
    let rl_key = format!("rate:login:{}", addr.ip());
    let n: i64 = redis::cmd("INCR")
        .arg(&rl_key)
        .query_async(&mut state.redis)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "redis err"))?;

    if n == 1 {
        let _: () = redis::cmd("EXPIRE")
            .arg(&rl_key)
            .arg(60)
            .query_async(&mut state.redis)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "redis err"))?;
    }

    if n > 10 {
        return Err((StatusCode::TOO_MANY_REQUESTS, "too many attempts"));
    }

    let row: Option<(Uuid, String)> =
        sqlx::query_as("SELECT id, password_hash FROM users WHERE username = $1")
            .bind(&req.username)
            .fetch_optional(&state.db)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db err"))?;

    let (user_id, hash_str) = match row {
        Some(r) => r,
        None => (Uuid::nil(), DUMMY_HASH.to_string()),
    };

    let parsed = PasswordHash::new(&hash_str)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "hash parse"))?;
    let ok = Argon2::default()
        .verify_password(req.password.as_bytes(), &parsed)
        .is_ok();

    if !ok || user_id.is_nil() {
        return Err((StatusCode::UNAUTHORIZED, "bad creds"));
    }

    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    let token = URL_SAFE_NO_PAD.encode(bytes);

    redis::cmd("SETEX")
        .arg(format!("session:{token}"))
        .arg(SESSION_TTL_SECS)
        .arg(user_id.to_string())
        .exec_async(&mut state.redis)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "redis err"))?;

    Ok(Json(LoginResp { token }))
}
