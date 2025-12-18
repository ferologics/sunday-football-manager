use crate::models::{EloSnapshot, Player, ELO_K_FACTOR, GD_MULTIPLIER_CAP};
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
/// Returns a map of player name -> EloSnapshot (before elo and delta)
pub fn calculate_elo_changes(
    team_a: &[Player],
    team_b: &[Player],
    score_a: i32,
    score_b: i32,
) -> HashMap<String, EloSnapshot> {
    let elo_a = average_elo(team_a);
    let elo_b = average_elo(team_b);

    let expected_a = expected_score(elo_a, elo_b);

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
        // Skip guests (id < 0)
        if p.id >= 0 {
            changes.insert(
                p.name.clone(),
                EloSnapshot {
                    before: p.elo,
                    delta: delta_a,
                },
            );
        }
    }

    for p in team_b {
        if p.id >= 0 {
            changes.insert(
                p.name.clone(),
                EloSnapshot {
                    before: p.elo,
                    delta: delta_b,
                },
            );
        }
    }

    changes
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
