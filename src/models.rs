use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::collections::HashMap;
use std::fmt;

/// Player tags for team balancing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Tag {
    Playmaker,
    Runner,
    Def,
    Atk,
    Gk,
}

impl Tag {
    /// All tags (excluding GK which has special handling)
    pub const ALL: &'static [Tag] = &[Tag::Playmaker, Tag::Runner, Tag::Def, Tag::Atk];

    /// Weight for team balancing (GK has no weight - special handling)
    pub fn weight(self) -> i32 {
        match self {
            Tag::Playmaker => 100,
            Tag::Runner => 80,
            Tag::Def => 40,
            Tag::Atk => 20,
            Tag::Gk => 0,
        }
    }

    /// Parse from string (case-insensitive)
    pub fn from_str(s: &str) -> Option<Tag> {
        match s.trim().to_uppercase().as_str() {
            "PLAYMAKER" => Some(Tag::Playmaker),
            "RUNNER" => Some(Tag::Runner),
            "DEF" => Some(Tag::Def),
            "ATK" => Some(Tag::Atk),
            "GK" => Some(Tag::Gk),
            _ => None,
        }
    }
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Tag::Playmaker => write!(f, "PLAYMAKER"),
            Tag::Runner => write!(f, "RUNNER"),
            Tag::Def => write!(f, "DEF"),
            Tag::Atk => write!(f, "ATK"),
            Tag::Gk => write!(f, "GK"),
        }
    }
}

/// Legacy constant for backwards compatibility with views
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
    /// Parse tags from comma-separated string into Tag enums
    pub fn tags(&self) -> Vec<Tag> {
        self.tags
            .split(',')
            .filter_map(Tag::from_str)
            .collect()
    }

    /// Check if player has a specific tag
    pub fn has_tag(&self, tag: Tag) -> bool {
        self.tags().contains(&tag)
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

