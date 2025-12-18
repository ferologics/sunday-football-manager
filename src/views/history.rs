use crate::models::{EloSnapshot, Match};
use crate::views::layout::{base, render_elo_delta};
use crate::{db, AppState};
use axum::{
    extract::State,
    response::{Html, IntoResponse},
};
use maud::{html, Markup};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

/// Build Elo timeline from matches for each player
fn build_elo_timeline(matches: &[Match], players: &[crate::models::Player]) -> serde_json::Value {
    // Matches are ordered newest first, reverse for chronological
    let matches_chrono: Vec<_> = matches.iter().rev().collect();

    // Track Elo for each player over time
    // Start with initial Elo (before first match = current - all deltas)
    let mut player_history: HashMap<String, Vec<(String, f32)>> = HashMap::new();

    // Calculate starting Elo for each player by working backwards
    let mut starting_elo: HashMap<String, f32> = players
        .iter()
        .map(|p| (p.name.clone(), p.elo))
        .collect();

    // Subtract all deltas to get starting Elo
    for m in matches {
        let snapshot: HashMap<String, EloSnapshot> = serde_json::from_value(m.elo_snapshot.clone())
            .unwrap_or_default();
        for (name, change) in &snapshot {
            if let Some(elo) = starting_elo.get_mut(name) {
                *elo -= change.delta;
            }
        }
    }

    // Now build timeline chronologically
    let mut current_elo = starting_elo.clone();

    for m in &matches_chrono {
        let date = m.played_at.format("%Y-%m-%d").to_string();
        let snapshot: HashMap<String, EloSnapshot> = serde_json::from_value(m.elo_snapshot.clone())
            .unwrap_or_default();

        // Update Elo for players in this match
        for (name, change) in &snapshot {
            current_elo.insert(name.clone(), change.before + change.delta);
            player_history
                .entry(name.clone())
                .or_default()
                .push((date.clone(), change.before + change.delta));
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
pub async fn page(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let matches = db::get_all_matches(&state.db).await.unwrap_or_default();
    let players = db::get_all_players(&state.db).await.unwrap_or_default();

    let chart_data = build_elo_timeline(&matches, &players);
    let chart_data_json = serde_json::to_string(&chart_data).unwrap_or_else(|_| "{}".to_string());

    let content = html! {
        h2 { "Match History" }

        // Elo evolution graph
        @if !matches.is_empty() {
            h3 { "Elo Evolution" }
            div style="position: relative; height: 400px; margin-bottom: 2rem;" {
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
