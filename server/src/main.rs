use axum::{
    Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use redis::aio::ConnectionManager;
use sqlx::PgPool;
use std::env;

mod auth;
mod login;
mod signup;

#[derive(Clone)]
struct AppState {
    db: PgPool,
    redis: ConnectionManager,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let redis_url = env::var("REDIS_URL").expect("REDIS_URL must be set");

    let db = PgPool::connect(&db_url)
        .await
        .expect("failed to connect to postgres");
    println!("Connected to postgres");

    let redis_client = redis::Client::open(redis_url).expect("invalid REDIS_URL");
    let redis = ConnectionManager::new(redis_client)
        .await
        .expect("failed to connect to resis");
    println!("Connected to redis");

    let state = AppState { db, redis };

    let app = Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/signup", post(signup::signup))
        .route("/login", post(login::login))
        .route("/me", get(auth::me))
        .with_state(state);

    let addr = "0.0.0.0:3000";
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    println!("listening on {addr}");

    axum::serve(listener, app).await.unwrap();
}

async fn health() -> &'static str {
    "ok"
}

async fn ready(State(mut state): State<AppState>) -> Result<&'static str, StatusCode> {
    sqlx::query("SELECT 1")
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    redis::cmd("PING")
        .query_async::<String>(&mut state.redis)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    Ok("ready")
}
