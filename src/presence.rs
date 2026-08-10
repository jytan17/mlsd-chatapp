use crate::contract::PresenceResp;
use crate::{AppState, auth::AuthUser};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use uuid::Uuid;

pub async fn get_presenced(
    AuthUser(_me): AuthUser,
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<PresenceResp>, (StatusCode, &'static str)> {
    let mut conn = state.redis.clone();
    let online: bool = redis::cmd("EXISTS")
        .arg(format!("presence:user:{user_id}"))
        .query_async(&mut conn)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "redis err"))?;
    Ok(Json(PresenceResp { user_id, online }))
}
