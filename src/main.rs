mod balance;
mod db;
mod elo;
mod models;
mod views;

use axum::{
    routing::{delete, get, post, put},
    Router,
};
use sqlx::PgPool;
use std::sync::Arc;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
}

#[tokio::main]
async fn main() {
    dotenvy::from_filename(".env.local").ok();
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    let pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to database");

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let state = Arc::new(AppState { db: pool });

    let router = Router::new()
        // Pages
        .route("/", get(views::match_day::page))
        .route("/roster", get(views::roster::page))
        .route("/record", get(views::record::page))
        .route("/history", get(views::history::page))
        // API - Players
        .route("/api/players", post(views::roster::create_player))
        .route("/api/players/{id}", put(views::roster::update_player))
        .route("/api/players/{id}", delete(views::roster::delete_player))
        .route("/api/seed", post(views::roster::seed_roster))
        // API - Match Day
        .route("/api/generate", post(views::match_day::generate_teams))
        .route("/api/shuffle", post(views::match_day::shuffle_teams))
        // API - Record
        .route("/api/record", post(views::record::submit_result))
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "8000".to_string());
    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind to address");

    axum::serve(listener, router)
        .await
        .expect("Server error");
}
