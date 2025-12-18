use crate::elo::average_elo;
use crate::models::{Player, Tag, TeamSplit};
use itertools::Itertools;
use rand::seq::SliceRandom;
use rand::thread_rng;

/// Calculate the cost of a team split
fn calculate_split_cost(team_a: &[Player], team_b: &[Player]) -> TeamSplit {
    let elo_a = average_elo(team_a);
    let elo_b = average_elo(team_b);
    let elo_diff = (elo_a - elo_b).abs();

    // Balance team "tag value" (sum of player tag values) instead of per-tag counts
    let tag_value_a: i32 = team_a.iter().map(|p| p.tag_value()).sum();
    let tag_value_b: i32 = team_b.iter().map(|p| p.tag_value()).sum();
    let tag_diff = (tag_value_a - tag_value_b).abs() as f32;

    TeamSplit {
        team_a: team_a.to_vec(),
        team_b: team_b.to_vec(),
        cost: elo_diff + tag_diff,
        elo_diff,
        tag_value_a,
        tag_value_b,
    }
}

/// Balance teams from a list of players
/// Returns the optimal split, or a random good split if randomize=true
pub fn balance_teams(players: &[Player], randomize: bool) -> Option<TeamSplit> {
    if players.len() < 2 {
        return None;
    }

    let team_size = players.len() / 2;

    // Identify goalkeepers
    let gks: Vec<_> = players
        .iter()
        .filter(|p| p.has_tag(Tag::Gk))
        .cloned()
        .collect();
    let non_gks: Vec<_> = players
        .iter()
        .filter(|p| !p.has_tag(Tag::Gk))
        .cloned()
        .collect();

    let mut all_splits: Vec<TeamSplit> = Vec::new();
    let mut best_split: Option<TeamSplit> = None;

    if gks.len() == 2 {
        // Force split: one GK per team
        let gk_a = &gks[0];
        let gk_b = &gks[1];
        let remaining_size = team_size.saturating_sub(1);

        if non_gks.len() >= remaining_size * 2 {
            for combo in non_gks.iter().cloned().combinations(remaining_size) {
                let mut team_a = vec![gk_a.clone()];
                team_a.extend(combo.iter().cloned());

                let team_b_rest: Vec<_> = non_gks
                    .iter()
                    .filter(|p| !combo.iter().any(|c| c.id == p.id))
                    .cloned()
                    .collect();
                let mut team_b = vec![gk_b.clone()];
                team_b.extend(team_b_rest);

                let split = calculate_split_cost(&team_a, &team_b);

                if best_split.is_none() || split.cost < best_split.as_ref().unwrap().cost {
                    best_split = Some(split.clone());
                }
                all_splits.push(split);
            }

            return pick_split(best_split, all_splits, randomize);
        }
    } else if gks.len() == 1 {
        // Single GK: assign to team A (deterministic), or random if randomize=true
        let gk = &gks[0];
        let gk_on_team_a = if randomize {
            rand::random::<bool>()
        } else {
            true
        };

        // Combo size depends on which team gets the GK
        // Team A needs (team_size - 1) non-GKs if GK is on team A
        // Team A needs team_size non-GKs if GK is on team B
        let combo_size = if gk_on_team_a {
            team_size - 1
        } else {
            team_size
        };

        for combo in non_gks.iter().cloned().combinations(combo_size) {
            let (team_a, team_b) = if gk_on_team_a {
                let mut a = vec![gk.clone()];
                a.extend(combo.iter().cloned());

                let b: Vec<_> = non_gks
                    .iter()
                    .filter(|p| !combo.iter().any(|c| c.id == p.id))
                    .cloned()
                    .collect();

                (a, b)
            } else {
                let a: Vec<_> = combo.to_vec();

                let mut b = vec![gk.clone()];
                b.extend(
                    non_gks
                        .iter()
                        .filter(|p| !combo.iter().any(|c| c.id == p.id))
                        .cloned(),
                );

                (a, b)
            };

            let split = calculate_split_cost(&team_a, &team_b);

            if best_split.is_none() || split.cost < best_split.as_ref().unwrap().cost {
                best_split = Some(split.clone());
            }
            all_splits.push(split);
        }

        return pick_split(best_split, all_splits, randomize);
    }

    // No GK special logic - standard brute force
    for combo in players.iter().cloned().combinations(team_size) {
        let team_a: Vec<_> = combo;
        let team_b: Vec<_> = players
            .iter()
            .filter(|p| !team_a.iter().any(|a| a.id == p.id))
            .cloned()
            .collect();

        let split = calculate_split_cost(&team_a, &team_b);

        if best_split.is_none() || split.cost < best_split.as_ref().unwrap().cost {
            best_split = Some(split.clone());
        }
        all_splits.push(split);
    }

    pick_split(best_split, all_splits, randomize)
}

/// Pick the final split - either best or random from near-optimal
fn pick_split(best: Option<TeamSplit>, all: Vec<TeamSplit>, randomize: bool) -> Option<TeamSplit> {
    let best = best?;

    if randomize && !all.is_empty() {
        // Pick randomly from splits within 10% of optimal (+ 1 for zero cost case)
        let threshold = best.cost * 1.1 + 1.0;
        let good_splits: Vec<_> = all.into_iter().filter(|s| s.cost <= threshold).collect();

        if !good_splits.is_empty() {
            let mut rng = thread_rng();
            return good_splits.choose(&mut rng).cloned();
        }
    }

    Some(best)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_player(id: i32, name: &str, elo: f32, tags: &str) -> Player {
        Player {
            id,
            name: name.to_string(),
            elo,
            tags: tags.to_string(),
            matches_played: 0,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn test_balance_two_players() {
        let players = vec![
            make_player(1, "Alice", 1200.0, ""),
            make_player(2, "Bob", 1200.0, ""),
        ];

        let split = balance_teams(&players, false).unwrap();
        assert_eq!(split.team_a.len(), 1);
        assert_eq!(split.team_b.len(), 1);
    }

    #[test]
    fn test_balance_with_tags() {
        let players = vec![
            make_player(1, "Alice", 1200.0, "PLAYMAKER"),
            make_player(2, "Bob", 1200.0, "PLAYMAKER"),
            make_player(3, "Carol", 1200.0, ""),
            make_player(4, "Dave", 1200.0, ""),
        ];

        let split = balance_teams(&players, false).unwrap();

        // Should split playmakers between teams
        let pm_a = split
            .team_a
            .iter()
            .filter(|p| p.has_tag(Tag::Playmaker))
            .count();
        let pm_b = split
            .team_b
            .iter()
            .filter(|p| p.has_tag(Tag::Playmaker))
            .count();
        assert_eq!(pm_a, 1);
        assert_eq!(pm_b, 1);
    }

    #[test]
    fn test_balance_two_gks() {
        let players = vec![
            make_player(1, "GK1", 1200.0, "GK"),
            make_player(2, "GK2", 1200.0, "GK"),
            make_player(3, "Player1", 1200.0, ""),
            make_player(4, "Player2", 1200.0, ""),
        ];

        let split = balance_teams(&players, false).unwrap();

        // Should force one GK per team
        let gk_a = split.team_a.iter().filter(|p| p.has_tag(Tag::Gk)).count();
        let gk_b = split.team_b.iter().filter(|p| p.has_tag(Tag::Gk)).count();
        assert_eq!(gk_a, 1);
        assert_eq!(gk_b, 1);
    }

    #[test]
    fn test_balance_one_gk_deterministic() {
        let players = vec![
            make_player(1, "GK", 1200.0, "GK"),
            make_player(2, "Player1", 1200.0, ""),
            make_player(3, "Player2", 1200.0, ""),
            make_player(4, "Player3", 1200.0, ""),
        ];

        // With randomize=false, should get same result every time
        let split1 = balance_teams(&players, false).unwrap();
        let split2 = balance_teams(&players, false).unwrap();

        // GK should be on team A (deterministic behavior)
        assert!(split1.team_a.iter().any(|p| p.has_tag(Tag::Gk)));
        assert!(split2.team_a.iter().any(|p| p.has_tag(Tag::Gk)));

        // Results should be identical
        let names1: Vec<_> = split1.team_a.iter().map(|p| &p.name).collect();
        let names2: Vec<_> = split2.team_a.iter().map(|p| &p.name).collect();
        assert_eq!(names1, names2);
    }

    #[test]
    fn test_balance_odd_players() {
        // 5 players should split into 2 and 3
        let players = vec![
            make_player(1, "A", 1200.0, ""),
            make_player(2, "B", 1200.0, ""),
            make_player(3, "C", 1200.0, ""),
            make_player(4, "D", 1200.0, ""),
            make_player(5, "E", 1200.0, ""),
        ];

        let split = balance_teams(&players, false).unwrap();

        // team_size = 5/2 = 2, so team_a has 2, team_b has 3
        assert_eq!(split.team_a.len(), 2);
        assert_eq!(split.team_b.len(), 3);
    }

    #[test]
    fn test_balance_insufficient_players() {
        // 0 players
        let empty: Vec<Player> = vec![];
        assert!(balance_teams(&empty, false).is_none());

        // 1 player
        let one = vec![make_player(1, "Alone", 1200.0, "")];
        assert!(balance_teams(&one, false).is_none());
    }

    #[test]
    fn test_balance_no_gks() {
        let players = vec![
            make_player(1, "A", 1400.0, ""),
            make_player(2, "B", 1200.0, ""),
            make_player(3, "C", 1200.0, ""),
            make_player(4, "D", 1000.0, ""),
        ];

        let split = balance_teams(&players, false).unwrap();

        // Should balance by Elo: 1400+1000 vs 1200+1200
        let elo_a: f32 = split.team_a.iter().map(|p| p.elo).sum();
        let elo_b: f32 = split.team_b.iter().map(|p| p.elo).sum();

        // Teams should have similar total Elo (within 200)
        assert!((elo_a - elo_b).abs() <= 200.0);
    }

    #[test]
    fn test_star_players_split_between_teams() {
        // Two "stars" with high tag value should end up on different teams
        let players = vec![
            make_player(1, "Star1", 1200.0, "PLAYMAKER,RUNNER,DEF"), // 50+40+20 = 110
            make_player(2, "Star2", 1200.0, "PLAYMAKER,RUNNER,DEF"), // 50+40+20 = 110
            make_player(3, "Role1", 1200.0, "DEF"),                  // 20
            make_player(4, "Role2", 1200.0, "DEF"),                  // 20
        ];

        let split = balance_teams(&players, false).unwrap();

        // Stars should be split between teams (Star1+Role2 vs Star2+Role1 or vice versa)
        // Total tag values: 110+20=130 per team vs putting stars together: 220 vs 40
        let star1_in_a = split.team_a.iter().any(|p| p.id == 1);
        let star2_in_a = split.team_a.iter().any(|p| p.id == 2);
        assert_ne!(star1_in_a, star2_in_a, "Stars should be on different teams");
    }
}
