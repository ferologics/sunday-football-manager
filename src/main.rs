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

#[shuttle_runtime::main]
async fn main(#[shuttle_shared_db::Postgres] pool: PgPool) -> shuttle_axum::ShuttleAxum {
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
        .route("/api/players/:id", put(views::roster::update_player))
        .route("/api/players/:id", delete(views::roster::delete_player))
        // API - Match Day
        .route("/api/generate", post(views::match_day::generate_teams))
        .route("/api/shuffle", post(views::match_day::shuffle_teams))
        // API - Record
        .route("/api/record", post(views::record::submit_result))
        .with_state(state);

    Ok(router.into())
}
