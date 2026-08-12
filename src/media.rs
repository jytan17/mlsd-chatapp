use crate::contract::{UploadReq, UploadResp};
use crate::{AppState, auth::AuthUser};
use aws_sdk_s3::{
    Client,
    config::{Credentials, Region},
    presigning::PresigningConfig,
};
use axum::{Json, extract::State, http::StatusCode};
use std::time::Duration;
use uuid::Uuid;

pub async fn make_s3() -> Client {
    let creds = Credentials::new("chat", "chatchat123", None, None, "static");
    let cfg = aws_sdk_s3::config::Builder::new()
        .region(Region::new("us-east-1"))
        .endpoint_url("http://localhost:9000")
        .credentials_provider(creds)
        .force_path_style(true)
        .build();

    Client::from_conf(cfg)
}

pub async fn presign_upload(
    AuthUser(_me): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<UploadReq>,
) -> Result<Json<UploadResp>, (StatusCode, &'static str)> {
    let media_id = Uuid::now_v7();
    let key = format!("{media_id}");
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
