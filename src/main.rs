use axum::routing::post;
use axum::{Json, Router, extract::State, http::StatusCode, response::Html, routing::get};
use redis::aio::ConnectionManager;
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use uuid::Uuid;

use crate::auth::AuthUser;

mod auth;
mod contract;
mod conversations;
mod fanout;
mod login;
mod media;
mod messages;
mod presence;
mod signup;
mod ws;

use crate::contract::MeResp;

pub type Hub = Arc<Mutex<HashMap<Uuid, Vec<Sender<String>>>>>;

#[derive(Clone)]
struct AppState {
    pub db: PgPool,
    pub redis: ConnectionManager,
    pub hub: Hub,
    pub pubsub_sink: fanout::SharedSink,
    pub subs: fanout::Subs,
    pub s3: aws_sdk_s3::Client,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");
    let redis_url = std::env::var("REDIS_URL").expect("REDIS_URL not set");

    let db = PgPoolOptions::new()
        .acquire_timeout(Duration::from_secs(2))
        .connect(&db_url)
        .await
        .expect("db connect");

    let redis_client = redis::Client::open(redis_url).expect("redis client");
    let redis = ConnectionManager::new_with_config(
        redis_client.clone(),
        redis::aio::ConnectionManagerConfig::new()
            .set_connection_timeout(Duration::from_secs(2))
            .set_response_timeout(Duration::from_secs(2)),
    )
    .await
    .expect("redis connect");

    let pubsub = redis_client
        .get_async_pubsub()
        .await
        .expect("pubsub connect");
    let (pubsub_sink, pubsub_stream) = pubsub.split();
    let pubsub_sink = Arc::new(tokio::sync::Mutex::new(pubsub_sink));
    let s3 = media::make_s3().await;
    let state = AppState {
        db,
        redis,
        hub: Arc::new(Mutex::new(HashMap::new())),
        pubsub_sink,
        subs: Arc::new(Mutex::new(HashMap::new())),
        s3: s3,
    };

    tokio::spawn(fanout::run_subscriber(
        redis_client.clone(),
        state.pubsub_sink.clone(),
        state.subs.clone(),
        state.hub.clone(),
        pubsub_stream,
    ));
    let app = Router::new()
        .route("/", get(index))
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/me", get(me))
        .route("/signup", post(signup::signup))
        .route("/login", post(login::login))
        .route(
            "/conversations",
            post(conversations::create_conversation).get(conversations::list_conversations),
        )
        .route(
            "/conversations/{id}/messages",
            post(messages::send_message).get(messages::list_messages),
        )
        .route("/conversations/{id}/read", post(messages::mark_read))
        .route("/ws", get(ws::ws_handler))
        .route("/presence/{user_id}", get(presence::get_presenced))
        .route("/media/upload-url", post(media::presign_upload))
        .route("/media/{media_id}/url", get(media::presign_download))
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".into());
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
    .unwrap();
}

async fn index() -> Html<&'static str> {
    Html(include_str!("../static/index.html"))
}

async fn health() -> &'static str {
    "ok"
}

async fn ready(State(mut state): State<AppState>) -> (StatusCode, &'static str) {
    let pg_ok = sqlx::query("SELECT 1").execute(&state.db).await.is_ok();
    let redis_ok = redis::cmd("PING")
        .query_async::<String>(&mut state.redis)
        .await
        .map(|r| r == "PONG")
        .unwrap_or(false);

    match (pg_ok, redis_ok) {
        (true, true) => (StatusCode::OK, "ready"),
        (false, _) => (StatusCode::SERVICE_UNAVAILABLE, "db down"),
        (_, false) => (StatusCode::SERVICE_UNAVAILABLE, "redis down"),
    }
}

async fn me(
    AuthUser(user_id): AuthUser,
    State(state): State<AppState>,
) -> Result<Json<MeResp>, (StatusCode, &'static str)> {
    let row: Option<(Uuid, String)> =
        sqlx::query_as("SELECT id, username FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(&state.db)
            .await
            .map_err(|_| (StatusCode::NOT_FOUND, "user gone"))?;

    let (id, username) = row.ok_or((StatusCode::NOT_FOUND, "user gone"))?;
    Ok(Json(MeResp { id, username }))
}
