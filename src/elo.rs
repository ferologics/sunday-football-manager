use crate::models::{EloSnapshot, Player, ELO_K_FACTOR, GD_MULTIPLIER_CAP, HANDICAP_PER_PLAYER};
use std::collections::HashMap;

/// Calculate expected score for team A
pub fn expected_score(elo_a: f32, elo_b: f32) -> f32 {
    1.0 / (1.0 + 10_f32.powf((elo_b - elo_a) / 400.0))
}

/// Calculate goal difference multiplier (capped)
pub fn goal_diff_multiplier(goal_diff: i32) -> f32 {
    if goal_diff <= 1 {
        1.0
    } else {
        (1.0 + (goal_diff - 1) as f32 * 0.5).min(GD_MULTIPLIER_CAP)
    }
}

/// Calculate average Elo of a team
pub fn average_elo(players: &[Player]) -> f32 {
    if players.is_empty() {
        return 0.0;
    }
    players.iter().map(|p| p.elo).sum::<f32>() / players.len() as f32
}

/// Calculate Elo changes for all players in a match
/// participation: map of player ID -> participation (0.0 to 1.0), defaults to 1.0
/// Returns a map of player ID -> EloSnapshot (before elo, delta, and participation)
pub fn calculate_elo_changes(
    team_a: &[Player],
    team_b: &[Player],
    score_a: i32,
    score_b: i32,
    participation: &HashMap<i32, f32>,
) -> HashMap<i32, EloSnapshot> {
    let elo_a = average_elo(team_a);
    let elo_b = average_elo(team_b);

    // Calculate effective team sizes based on participation
    let effective_a: f32 = team_a
        .iter()
        .map(|p| participation.get(&p.id).copied().unwrap_or(1.0))
        .sum();
    let effective_b: f32 = team_b
        .iter()
        .map(|p| participation.get(&p.id).copied().unwrap_or(1.0))
        .sum();

    // Calculate handicap: if Team A has fewer effective players, they're disadvantaged
    let player_diff = effective_b - effective_a;
    let handicap = player_diff * HANDICAP_PER_PLAYER;

    // Adjust Team A's Elo for expected score calculation
    let adjusted_elo_a = elo_a - handicap;
    let expected_a = expected_score(adjusted_elo_a, elo_b);

    let actual_a = if score_a > score_b {
        1.0
    } else if score_a < score_b {
        0.0
    } else {
        0.5
    };

    let gd = (score_a - score_b).abs();
    let multiplier = goal_diff_multiplier(gd);

    let delta_a = ELO_K_FACTOR * multiplier * (actual_a - expected_a);
    let delta_b = -delta_a; // Zero-sum

    let mut changes = HashMap::new();

    for p in team_a {
        let player_participation = participation.get(&p.id).copied().unwrap_or(1.0);
        changes.insert(
            p.id,
            EloSnapshot {
                before: p.elo,
                delta: delta_a,
                participation: player_participation,
            },
        );
    }

    for p in team_b {
        let player_participation = participation.get(&p.id).copied().unwrap_or(1.0);
        changes.insert(
            p.id,
            EloSnapshot {
                before: p.elo,
                delta: delta_b,
                participation: player_participation,
            },
        );
    }

    changes
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_player(id: i32, name: &str, elo: f32) -> Player {
        Player {
            id,
            name: name.to_string(),
            elo,
            tags: String::new(),
            matches_played: 0,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn test_expected_score_equal() {
        let expected = expected_score(1200.0, 1200.0);
        assert!((expected - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_expected_score_higher() {
        let expected = expected_score(1400.0, 1200.0);
        assert!(expected > 0.5);
    }

    #[test]
    fn test_goal_diff_multiplier() {
        assert_eq!(goal_diff_multiplier(0), 1.0);
        assert_eq!(goal_diff_multiplier(1), 1.0);
        assert_eq!(goal_diff_multiplier(2), 1.5);
        assert_eq!(goal_diff_multiplier(3), 2.0);
        assert_eq!(goal_diff_multiplier(10), GD_MULTIPLIER_CAP);
    }

    #[test]
    fn test_elo_changes_equal_teams() {
        let team_a = vec![
            make_player(1, "A1", 1200.0),
            make_player(2, "A2", 1200.0),
        ];
        let team_b = vec![
            make_player(3, "B1", 1200.0),
            make_player(4, "B2", 1200.0),
        ];
        let participation = HashMap::new(); // All 100%

        // Team A wins 2-1
        let changes = calculate_elo_changes(&team_a, &team_b, 2, 1, &participation);

        // All players should have Elo changes
        assert_eq!(changes.len(), 4);

        // Team A should gain, Team B should lose (keyed by player ID)
        assert!(changes.get(&1).unwrap().delta > 0.0);
        assert!(changes.get(&2).unwrap().delta > 0.0);
        assert!(changes.get(&3).unwrap().delta < 0.0);
        assert!(changes.get(&4).unwrap().delta < 0.0);

        // Zero-sum: total delta should be 0
        let total_delta: f32 = changes.values().map(|c| c.delta).sum();
        assert!((total_delta).abs() < 0.001);
    }

    #[test]
    fn test_elo_draw_result() {
        let team_a = vec![make_player(1, "A", 1200.0)];
        let team_b = vec![make_player(2, "B", 1200.0)];
        let participation = HashMap::new();

        // Draw 1-1
        let changes = calculate_elo_changes(&team_a, &team_b, 1, 1, &participation);

        // Equal Elo teams drawing should result in no change (keyed by player ID)
        let delta_a = changes.get(&1).unwrap().delta;
        let delta_b = changes.get(&2).unwrap().delta;

        assert!((delta_a).abs() < 0.001);
        assert!((delta_b).abs() < 0.001);
    }

    #[test]
    fn test_elo_large_upset() {
        // Much higher rated team loses
        let favorites = vec![make_player(1, "Favorite", 1600.0)];
        let underdogs = vec![make_player(2, "Underdog", 1000.0)];
        let participation = HashMap::new();

        // Underdog wins 3-0 (big upset with large goal diff)
        let changes = calculate_elo_changes(&favorites, &underdogs, 0, 3, &participation);

        // Favorites lose a lot, underdogs gain a lot (keyed by player ID)
        let fav_delta = changes.get(&1).unwrap().delta;
        let und_delta = changes.get(&2).unwrap().delta;

        assert!(fav_delta < -20.0); // Big loss
        assert!(und_delta > 20.0);  // Big gain
    }

    #[test]
    fn test_elo_handicap_6v7() {
        // Team A has 1 player, Team B has 2 (simulates 6v7)
        let team_a = vec![make_player(1, "A", 1200.0)];
        let team_b = vec![
            make_player(2, "B1", 1200.0),
            make_player(3, "B2", 1200.0),
        ];
        let participation = HashMap::new(); // All 100%

        // Draw - Team A should gain because they were handicapped
        let changes = calculate_elo_changes(&team_a, &team_b, 1, 1, &participation);

        // Team A had 1 player vs 2, so 100 Elo handicap
        // With handicap, Team A expected to lose, so draw = gain (keyed by player ID)
        let delta_a = changes.get(&1).unwrap().delta;
        assert!(delta_a > 0.0, "Short-handed team should gain Elo on draw");
    }

    #[test]
    fn test_elo_injury_partial_participation() {
        let team_a = vec![
            make_player(1, "A1", 1200.0),
            make_player(2, "A2", 1200.0), // injured
        ];
        let team_b = vec![
            make_player(3, "B1", 1200.0),
            make_player(4, "B2", 1200.0),
        ];

        let mut participation = HashMap::new();
        participation.insert(2, 0.5); // Player ID 2 (A2) played 50%

        // Team A wins
        let changes = calculate_elo_changes(&team_a, &team_b, 2, 1, &participation);

        // A2 (ID 2) should have 50% participation recorded
        assert_eq!(changes.get(&2).unwrap().participation, 0.5);

        // A1 (ID 1) should have 100% (default)
        assert_eq!(changes.get(&1).unwrap().participation, 1.0);
    }
}
