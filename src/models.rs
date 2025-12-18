use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
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
    /// Weight for team balancing (GK has no weight - special handling)
    pub fn weight(self) -> i32 {
        match self {
            Tag::Playmaker => 50,
            Tag::Runner => 40,
            Tag::Def => 20,
            Tag::Atk => 10,
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
pub const TAG_WEIGHTS: &[(&str, i32)] =
    &[("PLAYMAKER", 50), ("RUNNER", 40), ("DEF", 20), ("ATK", 10)];

pub const ELO_DEFAULT: f32 = 1200.0;
pub const ELO_K_FACTOR: f32 = 32.0;
pub const GD_MULTIPLIER_CAP: f32 = 2.5;
pub const HANDICAP_PER_PLAYER: f32 = 100.0; // Elo penalty per missing player-equivalent
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
        self.tags.split(',').filter_map(Tag::from_str).collect()
    }

    /// Check if player has a specific tag
    pub fn has_tag(&self, tag: Tag) -> bool {
        self.tags().contains(&tag)
    }

    /// Sum of tag weights for this player (for team balancing)
    pub fn tag_value(&self) -> i32 {
        self.tags().iter().map(|t| t.weight()).sum()
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
    pub team_a: Vec<i32>, // Player IDs
    pub team_b: Vec<i32>, // Player IDs
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
    #[serde(default = "default_participation")]
    pub participation: f32, // 0.0 to 1.0, default 1.0
}

fn default_participation() -> f32 {
    1.0
}

/// Result of team balancing
#[derive(Debug, Clone)]
pub struct TeamSplit {
    pub team_a: Vec<Player>,
    pub team_b: Vec<Player>,
    pub cost: f32,
    pub elo_diff: f32,
    pub tag_value_a: i32,
    pub tag_value_b: i32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_player(tags: &str) -> Player {
        Player {
            id: 1,
            name: "Test".to_string(),
            elo: 1200.0,
            tags: tags.to_string(),
            matches_played: 0,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn test_tag_from_str() {
        assert_eq!(Tag::from_str("PLAYMAKER"), Some(Tag::Playmaker));
        assert_eq!(Tag::from_str("playmaker"), Some(Tag::Playmaker));
        assert_eq!(Tag::from_str("  RUNNER  "), Some(Tag::Runner));
        assert_eq!(Tag::from_str("DEF"), Some(Tag::Def));
        assert_eq!(Tag::from_str("ATK"), Some(Tag::Atk));
        assert_eq!(Tag::from_str("GK"), Some(Tag::Gk));
        assert_eq!(Tag::from_str("INVALID"), None);
        assert_eq!(Tag::from_str(""), None);
    }

    #[test]
    fn test_tag_weight() {
        assert_eq!(Tag::Playmaker.weight(), 50);
        assert_eq!(Tag::Runner.weight(), 40);
        assert_eq!(Tag::Def.weight(), 20);
        assert_eq!(Tag::Atk.weight(), 10);
        assert_eq!(Tag::Gk.weight(), 0);
    }

    #[test]
    fn test_tag_display() {
        assert_eq!(format!("{}", Tag::Playmaker), "PLAYMAKER");
        assert_eq!(format!("{}", Tag::Runner), "RUNNER");
        assert_eq!(format!("{}", Tag::Def), "DEF");
        assert_eq!(format!("{}", Tag::Atk), "ATK");
        assert_eq!(format!("{}", Tag::Gk), "GK");
    }

    #[test]
    fn test_player_tags_parsing() {
        let player = make_player("PLAYMAKER,RUNNER,DEF");
        let tags = player.tags();
        assert_eq!(tags.len(), 3);
        assert!(tags.contains(&Tag::Playmaker));
        assert!(tags.contains(&Tag::Runner));
        assert!(tags.contains(&Tag::Def));
    }

    #[test]
    fn test_player_tags_empty() {
        let player = make_player("");
        assert!(player.tags().is_empty());
    }

    #[test]
    fn test_player_tags_invalid_mixed() {
        let player = make_player("PLAYMAKER,INVALID,DEF");
        let tags = player.tags();
        assert_eq!(tags.len(), 2);
        assert!(tags.contains(&Tag::Playmaker));
        assert!(tags.contains(&Tag::Def));
    }

    #[test]
    fn test_player_has_tag() {
        let player = make_player("GK,DEF");
        assert!(player.has_tag(Tag::Gk));
        assert!(player.has_tag(Tag::Def));
        assert!(!player.has_tag(Tag::Playmaker));
    }

    #[test]
    fn test_player_tag_value() {
        let star = make_player("PLAYMAKER,RUNNER,DEF"); // 50+40+20 = 110
        assert_eq!(star.tag_value(), 110);

        let gk = make_player("GK"); // 0
        assert_eq!(gk.tag_value(), 0);

        let empty = make_player("");
        assert_eq!(empty.tag_value(), 0);
    }

    #[test]
    fn test_elo_snapshot_default_participation() {
        let json = r#"{"before": 1200.0, "delta": 16.0}"#;
        let snapshot: EloSnapshot = serde_json::from_str(json).unwrap();
        assert_eq!(snapshot.participation, 1.0);
    }

    #[test]
    fn test_elo_snapshot_with_participation() {
        let json = r#"{"before": 1200.0, "delta": 16.0, "participation": 0.5}"#;
        let snapshot: EloSnapshot = serde_json::from_str(json).unwrap();
        assert_eq!(snapshot.participation, 0.5);
    }
}
