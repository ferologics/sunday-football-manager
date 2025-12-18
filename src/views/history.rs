use crate::auth::is_authenticated;
use crate::models::{EloSnapshot, Match, Player};
use crate::views::layout::{base, render_elo_delta, AuthState};
use crate::{db, AppState};
use axum::{
    extract::State,
    response::{Html, IntoResponse},
};
use axum_extra::extract::cookie::CookieJar;
use maud::{html, Markup};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

/// Build Elo timeline from matches for each player
fn build_elo_timeline(matches: &[Match], players: &[Player]) -> serde_json::Value {
    // Matches are ordered newest first, reverse for chronological
    let matches_chrono: Vec<_> = matches.iter().rev().collect();

    // Build ID → name map
    let id_to_name: HashMap<i32, &str> = players.iter().map(|p| (p.id, p.name.as_str())).collect();

    // Track Elo for each player over time (keyed by name for chart display)
    let mut player_history: HashMap<String, Vec<(String, f32)>> = HashMap::new();

    // Calculate starting Elo for each player by working backwards
    let mut starting_elo: HashMap<i32, f32> = players.iter().map(|p| (p.id, p.elo)).collect();

    // Subtract all deltas (effective = delta * participation) to get starting Elo
    for m in matches {
        let snapshot: HashMap<i32, EloSnapshot> =
            serde_json::from_value(m.elo_snapshot.clone()).unwrap_or_default();
        for (player_id, change) in &snapshot {
            if let Some(elo) = starting_elo.get_mut(player_id) {
                let effective_delta = change.delta * change.participation;
                *elo -= effective_delta;
            }
        }
    }

    // Now build timeline chronologically
    let mut current_elo = starting_elo.clone();

    for m in &matches_chrono {
        let date = m.played_at.format("%Y-%m-%d").to_string();
        let snapshot: HashMap<i32, EloSnapshot> =
            serde_json::from_value(m.elo_snapshot.clone()).unwrap_or_default();

        // Update Elo for players in this match (using effective delta)
        for (player_id, change) in &snapshot {
            let effective_delta = change.delta * change.participation;
            let new_elo = change.before + effective_delta;
            current_elo.insert(*player_id, new_elo);
            if let Some(name) = id_to_name.get(player_id) {
                player_history
                    .entry((*name).to_string())
                    .or_default()
                    .push((date.clone(), new_elo));
            }
        }
    }

    // Convert to chart.js format
    let mut datasets: Vec<serde_json::Value> = Vec::new();
    let colors = [
        "#3498db", "#e74c3c", "#2ecc71", "#9b59b6", "#f39c12",
        "#1abc9c", "#e67e22", "#34495e", "#16a085", "#c0392b",
        "#8e44ad", "#27ae60", "#d35400", "#2980b9", "#f1c40f",
        "#7f8c8d", "#95a5a6", "#d63031", "#00b894", "#0984e3",
        "#6c5ce7", "#fd79a8",
    ];

    for (i, player) in players.iter().enumerate() {
        if let Some(history) = player_history.get(&player.name) {
            let color = colors[i % colors.len()];
            datasets.push(json!({
                "label": player.name,
                "data": history.iter().map(|(date, elo)| json!({ "x": date, "y": elo })).collect::<Vec<_>>(),
                "borderColor": color,
                "backgroundColor": color,
                "fill": false,
                "tension": 0.1
            }));
        }
    }

    json!({ "datasets": datasets })
}

/// History page - match history
pub async fn page(State(state): State<Arc<AppState>>, jar: CookieJar) -> impl IntoResponse {
    let matches = db::get_all_matches(&state.db).await.unwrap_or_default();
    let players = db::get_all_players(&state.db).await.unwrap_or_default();
    let auth = AuthState::new(state.auth_password.is_some(), is_authenticated(&jar, &state));

    // Build player ID → name map for display
    let player_names: HashMap<i32, String> = players.iter().map(|p| (p.id, p.name.clone())).collect();

    let chart_data = build_elo_timeline(&matches, &players);
    let chart_data_json = serde_json::to_string(&chart_data).unwrap_or_else(|_| "{}".to_string());

    let content = html! {
        h2 { "Match History" }

        // Elo evolution graph
        @if !matches.is_empty() {
            h3 { "Elo Evolution" }
            div class="chart-container" {
                canvas id="elo-chart" {}
            }

            script src="https://cdn.jsdelivr.net/npm/chart.js" {}
            script {
                (maud::PreEscaped(format!(r#"
                    const chartData = {chart_data};
                    const ctx = document.getElementById('elo-chart').getContext('2d');
                    new Chart(ctx, {{
                        type: 'line',
                        data: chartData,
                        options: {{
                            responsive: true,
                            maintainAspectRatio: false,
                            plugins: {{
                                legend: {{
                                    position: 'bottom',
                                    labels: {{
                                        usePointStyle: true,
                                        padding: 15
                                    }},
                                    onClick: function(e, legendItem, legend) {{
                                        const index = legendItem.datasetIndex;
                                        const ci = legend.chart;
                                        const meta = ci.getDatasetMeta(index);
                                        meta.hidden = meta.hidden === null ? !ci.data.datasets[index].hidden : null;
                                        ci.update();
                                    }}
                                }},
                                tooltip: {{
                                    mode: 'index',
                                    intersect: false
                                }}
                            }},
                            scales: {{
                                x: {{
                                    type: 'category',
                                    title: {{
                                        display: true,
                                        text: 'Match Date'
                                    }}
                                }},
                                y: {{
                                    title: {{
                                        display: true,
                                        text: 'Elo Rating'
                                    }}
                                }}
                            }},
                            interaction: {{
                                mode: 'nearest',
                                axis: 'x',
                                intersect: false
                            }}
                        }}
                    }});
                "#, chart_data = chart_data_json)))
            }

            hr;
        }

        // Match log
        h3 { "Match Log" }
        @if matches.is_empty() {
            p { "No matches recorded yet." }
        } @else {
            @for m in &matches {
                (render_match(m, &player_names))
            }
        }
    };

    Html(base("History", "history", &auth, content).into_string())
}

/// Render a single match as a collapsible card
fn render_match(m: &Match, player_names: &HashMap<i32, String>) -> Markup {
    let result_text = if m.score_a > m.score_b {
        "Team A wins"
    } else if m.score_b > m.score_a {
        "Team B wins"
    } else {
        "Draw"
    };

    // Parse Elo snapshot (ID-keyed format)
    let snapshot: HashMap<i32, EloSnapshot> =
        serde_json::from_value(m.elo_snapshot.clone()).unwrap_or_default();

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
                        @for player_id in &m.team_a {
                            @let name = player_names.get(player_id).map(|s| s.as_str()).unwrap_or("Unknown");
                            li {
                                (name)
                                @if let Some(change) = snapshot.get(player_id) {
                                    " "
                                    @let effective_delta = change.delta * change.participation;
                                    (render_elo_delta(effective_delta))
                                    @if change.participation < 1.0 {
                                        span class="secondary participation-pct" {
                                            " (" (format!("{:.0}%", change.participation * 100.0)) ")"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Team B
                div {
                    h4 { "Team B" }
                    ul class="player-list" {
                        @for player_id in &m.team_b {
                            @let name = player_names.get(player_id).map(|s| s.as_str()).unwrap_or("Unknown");
                            li {
                                (name)
                                @if let Some(change) = snapshot.get(player_id) {
                                    " "
                                    @let effective_delta = change.delta * change.participation;
                                    (render_elo_delta(effective_delta))
                                    @if change.participation < 1.0 {
                                        span class="secondary participation-pct" {
                                            " (" (format!("{:.0}%", change.participation * 100.0)) ")"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
