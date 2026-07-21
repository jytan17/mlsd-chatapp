use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
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
        return Err((StatusCode::BAD_REQUEST, "username 3..32".into()));
    }
    if req.password.len() < 8 {
        return Err((
            StatusCode::BAD_REQUEST,
            "passwor too short (must be >= 8 chars)".into(),
        ));
    }

    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(req.password.as_bytes(), &salt)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "hash fail".into()))?
        .to_string();

    let id = Uuid::now_v7();
    let res = sqlx::query("INSERT INTO users (id, username, password_hash) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(&req.username)
        .bind(&hash)
        .execute(&state.db)
        .await;

    match res {
        Ok(_) => Ok((
            StatusCode::CREATED,
            Json(SignupResp {
                id,
                username: req.username,
            }),
        )),
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
            Err((StatusCode::CONFLICT, "username taken".into()))
        }
        Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, "db err".into())),
    }
}
