use crate::models::{EloSnapshot, Match};
use crate::views::layout::{base, render_elo_delta};
use crate::{db, AppState};
use axum::{
    extract::State,
    response::{Html, IntoResponse},
};
use maud::{html, Markup};
use std::collections::HashMap;
use std::sync::Arc;

/// History page - match history
pub async fn page(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let matches = db::get_all_matches(&state.db).await.unwrap_or_default();
    let players = db::get_all_players(&state.db).await.unwrap_or_default();

    let content = html! {
        h2 { "Match History" }

        // Current standings
        h3 { "Current Standings" }
        @if players.is_empty() {
            p { "No players yet." }
        } @else {
            table {
                thead {
                    tr {
                        th { "#" }
                        th { "Player" }
                        th { "Elo" }
                        th { "Matches" }
                    }
                }
                tbody {
                    @for (i, player) in players.iter().enumerate() {
                        tr {
                            td { (i + 1) }
                            td { (player.name) }
                            td { (format!("{:.0}", player.elo)) }
                            td { (player.matches_played) }
                        }
                    }
                }
            }
        }

        hr;

        // Match log
        h3 { "Match Log" }
        @if matches.is_empty() {
            p { "No matches recorded yet." }
        } @else {
            @for m in &matches {
                (render_match(m))
            }
        }
    };

    Html(base("History", "history", content).into_string())
}

/// Render a single match as a collapsible card
fn render_match(m: &Match) -> Markup {
    let result_text = if m.score_a > m.score_b {
        "Team A wins"
    } else if m.score_b > m.score_a {
        "Team B wins"
    } else {
        "Draw"
    };

    // Parse Elo snapshot
    let snapshot: HashMap<String, EloSnapshot> = serde_json::from_value(m.elo_snapshot.clone())
        .unwrap_or_default();

    html! {
        details {
            summary {
                strong { (m.played_at.format("%Y-%m-%d")) }
                " - "
                (m.score_a) " : " (m.score_b)
                " (" (result_text) ")"
            }

            div class="team-grid" {
                // Team A
                div {
                    h4 { "Team A" }
                    ul class="player-list" {
                        @for name in &m.team_a {
                            li {
                                (name)
                                @if let Some(change) = snapshot.get(name) {
                                    " "
                                    (render_elo_delta(change.delta))
                                }
                            }
                        }
                    }
                }

                // Team B
                div {
                    h4 { "Team B" }
                    ul class="player-list" {
                        @for name in &m.team_b {
                            li {
                                (name)
                                @if let Some(change) = snapshot.get(name) {
                                    " "
                                    (render_elo_delta(change.delta))
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
