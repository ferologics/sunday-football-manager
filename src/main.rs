mod auth;
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
use tower_http::trace::TraceLayer;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub auth_password: Option<String>,
}

#[tokio::main]
async fn main() {
    dotenvy::from_filename(".env.local").ok();
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

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

    let auth_password = std::env::var("AUTH_PASSWORD").ok();
    if auth_password.is_some() {
        tracing::info!("Auth password configured - login required for mutations");
    } else {
        tracing::warn!("No AUTH_PASSWORD set - site is unprotected");
    }

    let state = Arc::new(AppState { db: pool, auth_password });

    let router = Router::new()
        // Pages
        .route("/", get(views::match_day::page))
        .route("/roster", get(views::roster::page))
        .route("/record", get(views::record::page))
        .route("/history", get(views::history::page))
        // Auth
        .route("/api/login", post(auth::login))
        .route("/api/logout", post(auth::logout))
        // API - Players
        .route("/api/players", post(views::roster::create_player))
        .route("/api/players/{id}", put(views::roster::update_player))
        .route("/api/players/{id}", delete(views::roster::delete_player))
        // API - Team Generator
        .route("/api/generate", post(views::match_day::generate_teams))
        .route("/api/shuffle", post(views::match_day::shuffle_teams))
        // API - Record
        .route("/api/record", post(views::record::submit_result))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "8000".to_string());
    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("Starting server on http://localhost:{}", port);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind to address");

    axum::serve(listener, router)
        .await
        .expect("Server error");
}
