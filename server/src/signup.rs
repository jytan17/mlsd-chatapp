use argon2::{
    Argon2, PasswordHasher,
    password_hash::{SaltString, rand_core::OsRng},
};
use axum::{Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AppState;

#[derive(Deserialize)]
pub struct SignupReq {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct SignupResp {
    pub id: Uuid,
    pub username: String,
}

pub async fn signup(
    State(state): State<AppState>,
    Json(req): Json<SignupReq>,
) -> Result<(StatusCode, Json<SignupResp>), (StatusCode, String)> {
    if req.username.len() < 3 || req.username.len() > 32 {
        return Err((
            StatusCode::BAD_REQUEST,
            "User name length must be between 3 and 32 characters".into(),
        ));
    }

    if req.password.len() < 8 {
        return Err((StatusCode::BAD_REQUEST, "password is too short".into()));
    }

    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(req.password.as_bytes(), &salt)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("hash: {e}")))?
        .to_string();

    let id = Uuid::now_v7();
    let result = sqlx::query("INSERT INTO users (id, username, password_hash) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(&req.username)
        .bind(&hash)
        .execute(&state.db)
        .await;

    match result {
        Ok(_) => Ok((
            StatusCode::CREATED,
            Json(SignupResp {
                id,
                username: req.username,
            }),
        )),
        Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
            Err((StatusCode::CONFLICT, "username taken".into()))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, format!("db: {e}"))),
    }
}
