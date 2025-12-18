use crate::models::{Match, NewPlayer, Player, UpdatePlayer, ELO_DEFAULT};
use sqlx::PgPool;

/// Get all players ordered by Elo (descending)
pub async fn get_all_players(pool: &PgPool) -> Result<Vec<Player>, sqlx::Error> {
    sqlx::query_as::<_, Player>(
        "SELECT id, name, elo, tags, matches_played, created_at FROM players ORDER BY elo DESC",
    )
    .fetch_all(pool)
    .await
}

/// Get players by IDs
pub async fn get_players_by_ids(pool: &PgPool, ids: &[i32]) -> Result<Vec<Player>, sqlx::Error> {
    sqlx::query_as::<_, Player>(
        "SELECT id, name, elo, tags, matches_played, created_at FROM players WHERE id = ANY($1)",
    )
    .bind(ids)
    .fetch_all(pool)
    .await
}

/// Get a single player by ID
#[allow(dead_code)]
pub async fn get_player(pool: &PgPool, id: i32) -> Result<Option<Player>, sqlx::Error> {
    sqlx::query_as::<_, Player>(
        "SELECT id, name, elo, tags, matches_played, created_at FROM players WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

/// Get a single player by name
#[allow(dead_code)]
pub async fn get_player_by_name(pool: &PgPool, name: &str) -> Result<Option<Player>, sqlx::Error> {
    sqlx::query_as::<_, Player>(
        "SELECT id, name, elo, tags, matches_played, created_at FROM players WHERE name = $1",
    )
    .bind(name)
    .fetch_optional(pool)
    .await
}

/// Create a new player
pub async fn create_player(pool: &PgPool, player: &NewPlayer) -> Result<Player, sqlx::Error> {
    let elo = player.elo.unwrap_or(ELO_DEFAULT);
    let tags = player.tags.as_deref().unwrap_or("");

    sqlx::query_as::<_, Player>(
        "INSERT INTO players (name, elo, tags) VALUES ($1, $2, $3)
         RETURNING id, name, elo, tags, matches_played, created_at",
    )
    .bind(&player.name)
    .bind(elo)
    .bind(tags)
    .fetch_one(pool)
    .await
}

/// Update a player
pub async fn update_player(
    pool: &PgPool,
    id: i32,
    update: &UpdatePlayer,
) -> Result<Option<Player>, sqlx::Error> {
    sqlx::query_as::<_, Player>(
        "UPDATE players SET elo = $1, tags = $2 WHERE id = $3
         RETURNING id, name, elo, tags, matches_played, created_at",
    )
    .bind(update.elo)
    .bind(&update.tags)
    .bind(id)
    .fetch_optional(pool)
    .await
}

/// Update player Elo and match count after a match
pub async fn update_player_elo(pool: &PgPool, name: &str, new_elo: f32) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE players SET elo = $1, matches_played = matches_played + 1 WHERE name = $2")
        .bind(new_elo)
        .bind(name)
        .execute(pool)
        .await?;
    Ok(())
}

/// Delete a player
pub async fn delete_player(pool: &PgPool, id: i32) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM players WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

/// Get all matches ordered by date (most recent first)
pub async fn get_all_matches(pool: &PgPool) -> Result<Vec<Match>, sqlx::Error> {
    sqlx::query_as::<_, Match>(
        "SELECT id, played_at, team_a, team_b, score_a, score_b, elo_snapshot, created_at
         FROM matches ORDER BY played_at DESC, created_at DESC",
    )
    .fetch_all(pool)
    .await
}

/// Create a new match record
pub async fn create_match(
    pool: &PgPool,
    team_a: &[String],
    team_b: &[String],
    score_a: i32,
    score_b: i32,
    elo_snapshot: serde_json::Value,
) -> Result<Match, sqlx::Error> {
    sqlx::query_as::<_, Match>(
        "INSERT INTO matches (team_a, team_b, score_a, score_b, elo_snapshot)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id, played_at, team_a, team_b, score_a, score_b, elo_snapshot, created_at",
    )
    .bind(team_a)
    .bind(team_b)
    .bind(score_a)
    .bind(score_b)
    .bind(elo_snapshot)
    .fetch_one(pool)
    .await
}
