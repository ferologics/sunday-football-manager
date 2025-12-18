use crate::elo::average_elo;
use crate::models::{Player, TeamSplit, TAG_WEIGHTS};
use itertools::Itertools;
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::collections::HashMap;

/// Count how many players have a specific tag
fn count_tags(players: &[Player], tag: &str) -> i32 {
    players.iter().filter(|p| p.has_tag(tag)).count() as i32
}

/// Calculate the cost of a team split
fn calculate_split_cost(team_a: &[Player], team_b: &[Player]) -> TeamSplit {
    let elo_a = average_elo(team_a);
    let elo_b = average_elo(team_b);
    let elo_diff = (elo_a - elo_b).abs();

    let mut tag_costs = HashMap::new();
    let mut total_tag_cost = 0.0;

    for (tag, weight) in TAG_WEIGHTS {
        let count_a = count_tags(team_a, tag);
        let count_b = count_tags(team_b, tag);
        let diff = (count_a - count_b).abs();
        tag_costs.insert(tag.to_string(), diff);
        total_tag_cost += diff as f32 * *weight as f32;
    }

    TeamSplit {
        team_a: team_a.to_vec(),
        team_b: team_b.to_vec(),
        cost: elo_diff + total_tag_cost,
        elo_diff,
        tag_costs,
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
    let gks: Vec<_> = players.iter().filter(|p| p.has_tag("GK")).cloned().collect();
    let non_gks: Vec<_> = players.iter().filter(|p| !p.has_tag("GK")).cloned().collect();

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
        // Random assignment for single GK
        let gk = &gks[0];
        let gk_on_team_a = rand::random::<bool>();

        for combo in non_gks.iter().cloned().combinations(team_size - 1) {
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

            // Ensure teams are balanced in size
            if (team_a.len() as i32 - team_b.len() as i32).abs() > 1 {
                continue;
            }

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
fn pick_split(
    best: Option<TeamSplit>,
    all: Vec<TeamSplit>,
    randomize: bool,
) -> Option<TeamSplit> {
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
        let pm_a = split.team_a.iter().filter(|p| p.has_tag("PLAYMAKER")).count();
        let pm_b = split.team_b.iter().filter(|p| p.has_tag("PLAYMAKER")).count();
        assert_eq!(pm_a, 1);
        assert_eq!(pm_b, 1);
    }
}
