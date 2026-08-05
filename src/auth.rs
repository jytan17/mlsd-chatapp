use axum::{
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
};
use uuid::Uuid;

use crate::AppState;

pub struct AuthUser(pub Uuid);

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // Prefer `Authorization: Bearer <token>` (curl/websocat).
        // Fall back to `?token=<token>` query param — browsers can't set
        // headers on a WebSocket handshake, so the WS client passes it in the URL.
        let header_token = parts
            .headers
            .get("authorization")
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.strip_prefix("Bearer "));

        let query_token = parts.uri.query().and_then(|q| {
            q.split('&')
                .find_map(|kv| kv.strip_prefix("token="))
        });

        let token =
            header_token
                .or(query_token)
                .ok_or((StatusCode::UNAUTHORIZED, "no token"))?;

        let mut redis_conn = state.redis.clone();
        let user_id: Option<String> = redis::cmd("GET")
            .arg(format!("session:{token}"))
            .query_async(&mut redis_conn)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "redis err"))?;

        let user_id = user_id.ok_or((StatusCode::UNAUTHORIZED, "bad token"))?;
        let uuid = Uuid::parse_str(&user_id)
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "bad uuid"))?;

        Ok(AuthUser(uuid))
    }
}
