use axum::routing::post;
use axum::{Router, extract::State, http::StatusCode, routing::get};
use redis::aio::ConnectionManager;
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::time::Duration;

mod login;
mod signup;

#[derive(Clone)]
struct AppState {
    pub db: PgPool,
    pub redis: ConnectionManager,
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
        redis_client,
        redis::aio::ConnectionManagerConfig::new()
            .set_connection_timeout(Duration::from_secs(2))
            .set_response_timeout(Duration::from_secs(2)),
    )
    .await
    .expect("redis connect");

    let state = AppState { db, redis };
    let app = Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/signup", post(signup::signup))
        .route("/login", post(login::login))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
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
