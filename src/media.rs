use crate::contract::{MediaUrlResp, UploadReq, UploadResp};
use crate::{AppState, auth::AuthUser};
use aws_sdk_s3::{
    Client,
    config::{Credentials, Region},
    presigning::PresigningConfig,
};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use std::time::Duration;
use uuid::Uuid;

pub async fn make_s3() -> Client {
    let creds = Credentials::new("chat", "chatchat123", None, None, "static");
    let cfg = aws_sdk_s3::config::Builder::new()
        .behavior_version(aws_sdk_s3::config::BehaviorVersion::latest())
        .region(Region::new("us-east-1"))
        .endpoint_url("http://localhost:9000")
        .credentials_provider(creds)
        .force_path_style(true)
        .build();

    Client::from_conf(cfg)
}

pub async fn presign_upload(
    AuthUser(me): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<UploadReq>,
) -> Result<Json<UploadResp>, (StatusCode, &'static str)> {
    let media_id = Uuid::now_v7();
    let key = format!("{media_id}");
    sqlx::query("INSERT INTO media (id, uploader_id, content_type) VALUES ($1, $2, $3)")
        .bind(media_id)
        .bind(me)
        .bind(&req.content_type)
        .execute(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db err"))?;

    let presign = PresigningConfig::expires_in(Duration::from_secs(300))
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "presign cfg"))?;

    let req_out = state
        .s3
        .put_object()
        .bucket("media")
        .key(&key)
        .content_type(req.content_type)
        .presigned(presign)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "presign"))?;

    Ok(Json(UploadResp {
        media_id,
        upload_url: req_out.uri().to_string(),
    }))
}

pub async fn presign_download(
    AuthUser(me): AuthUser,
    State(state): State<AppState>,
    Path(media_id): Path<Uuid>,
) -> Result<Json<MediaUrlResp>, (StatusCode, &'static str)> {
    let allowed: Option<i32> = sqlx::query_scalar(
        "
        SELECT 1
        FROM messages m
        JOIN conversation_members cm ON cm.conversation_id = m.conversation_id
        WHERE m.media_id = $1 AND cm.user_id = $2
        LIMIT 1",
    )
    .bind(media_id)
    .bind(me)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db err"))?;

    if allowed.is_none() {
        return Err((StatusCode::FORBIDDEN, "no access"));
    }

    let presign = PresigningConfig::expires_in(Duration::from_secs(300))
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db err"))?;

    let req_out = state
        .s3
        .get_object()
        .bucket("media")
        .key(media_id.to_string())
        .presigned(presign)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "presigh"))?;

    Ok(Json(MediaUrlResp {
        url: req_out.uri().to_string(),
    }))
}
