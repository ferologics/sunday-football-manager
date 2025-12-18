use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::collections::HashMap;

/// Tag weights for team balancing
pub const TAG_WEIGHTS: &[(&str, i32)] = &[
    ("PLAYMAKER", 100),
    ("RUNNER", 80),
    ("DEF", 40),
    ("ATK", 20),
];

pub const ELO_DEFAULT: f32 = 1200.0;
pub const ELO_K_FACTOR: f32 = 32.0;
pub const GD_MULTIPLIER_CAP: f32 = 2.5;
pub const MAX_PLAYERS: usize = 14;
pub const MAX_PER_TEAM: usize = MAX_PLAYERS / 2;

/// Player from database
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Player {
    pub id: i32,
    pub name: String,
    pub elo: f32,
    pub tags: String,
    pub matches_played: i32,
    pub created_at: DateTime<Utc>,
}

impl Player {
    /// Parse tags from comma-separated string
    pub fn tag_list(&self) -> Vec<&str> {
        self.tags
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect()
    }

    /// Check if player has a specific tag
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tag_list()
            .iter()
            .any(|t| t.eq_ignore_ascii_case(tag))
    }
}

/// Form data for creating a new player
#[derive(Debug, Deserialize)]
pub struct NewPlayer {
    pub name: String,
    pub elo: Option<f32>,
    pub tags: Option<String>,
}

/// Form data for updating a player
#[derive(Debug, Deserialize)]
pub struct UpdatePlayer {
    pub elo: f32,
    pub tags: String,
}

/// Match from database
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Match {
    pub id: i32,
    pub played_at: NaiveDate,
    pub team_a: Vec<String>,
    pub team_b: Vec<String>,
    pub score_a: i32,
    pub score_b: i32,
    pub elo_snapshot: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

/// Elo snapshot entry for a player
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EloSnapshot {
    pub before: f32,
    pub delta: f32,
}

/// Result of team balancing
#[derive(Debug, Clone)]
pub struct TeamSplit {
    pub team_a: Vec<Player>,
    pub team_b: Vec<Player>,
    pub cost: f32,
    pub elo_diff: f32,
    pub tag_costs: HashMap<String, i32>,
}

/// Form data for recording a match result
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct RecordMatch {
    pub team_a: Vec<String>,
    pub team_b: Vec<String>,
    pub score_a: i32,
    pub score_b: i32,
}

/// Form data for generating teams
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct GenerateTeams {
    pub player_ids: Vec<i32>,
}

/// Guest player (not in database, temporary for a match)
/// TODO: Implement guest support in UI if needed
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Guest {
    pub name: String,
    pub elo: f32,
    pub tags: String,
}

#[allow(dead_code)]
impl Guest {
    pub fn to_player(&self, guest_id: i32) -> Player {
        Player {
            id: -guest_id, // Unique negative ID for each guest
            name: format!("[G] {}", self.name),
            elo: self.elo,
            tags: self.tags.clone(),
            matches_played: 0,
            created_at: Utc::now(),
        }
    }
}
