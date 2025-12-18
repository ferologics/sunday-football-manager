use crate::elo::calculate_elo_changes;
use crate::models::{EloSnapshot, Player, MAX_PER_TEAM};
use crate::views::layout::{base, render_elo_delta};
use crate::{db, AppState};
use axum::{
    extract::State,
    response::{Html, IntoResponse},
    Form,
};
use maud::{html, Markup};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

/// Record Result page
pub async fn page(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let players = db::get_all_players(&state.db).await.unwrap_or_default();

    let content = html! {
        h2 { "Record Match Result" }

        form id="record-form" hx-post="/api/record" hx-target="#result-display" {
            // Team selection
            div class="team-grid" {
                // Team A
                fieldset {
                    legend { "Team A (max " (MAX_PER_TEAM) ")" }
                    div class="checkbox-grid" {
                        @for player in &players {
                            label {
                                input
                                    type="checkbox"
                                    name="team_a"
                                    value=(player.name)
                                    class="team-a-checkbox";
                                (player.name) " (" (format!("{:.0}", player.elo)) ")"
                            }
                        }
                    }
                }

                // Team B
                fieldset {
                    legend { "Team B (max " (MAX_PER_TEAM) ")" }
                    div class="checkbox-grid" {
                        @for player in &players {
                            label {
                                input
                                    type="checkbox"
                                    name="team_b"
                                    value=(player.name)
                                    class="team-b-checkbox";
                                (player.name) " (" (format!("{:.0}", player.elo)) ")"
                            }
                        }
                    }
                }
            }

            hr;

            // Score input
            h3 { "Score" }
            div class="grid" style="align-items: center;" {
                div {
                    label { "Team A" }
                    input type="number" name="score_a" value="0" min="0" max="50" required;
                }
                div style="text-align: center; font-size: 2rem;" { "-" }
                div {
                    label { "Team B" }
                    input type="number" name="score_b" value="0" min="0" max="50" required;
                }
            }

            button type="submit" { "Submit Result" }
        }

        // Result display area
        div id="result-display" {}

        // Script to enforce max per team and prevent overlap
        script {
            (maud::PreEscaped(format!(r#"
                const maxPerTeam = {};

                function updateCheckboxes() {{
                    const teamA = document.querySelectorAll('.team-a-checkbox:checked');
                    const teamB = document.querySelectorAll('.team-b-checkbox:checked');

                    const teamAValues = new Set([...teamA].map(c => c.value));
                    const teamBValues = new Set([...teamB].map(c => c.value));

                    // Disable unchecked Team A boxes if at max
                    document.querySelectorAll('.team-a-checkbox').forEach(c => {{
                        if (!c.checked) {{
                            c.disabled = teamA.length >= maxPerTeam || teamBValues.has(c.value);
                        }}
                    }});

                    // Disable unchecked Team B boxes if at max or in Team A
                    document.querySelectorAll('.team-b-checkbox').forEach(c => {{
                        if (!c.checked) {{
                            c.disabled = teamB.length >= maxPerTeam || teamAValues.has(c.value);
                        }}
                    }});
                }}

                document.querySelectorAll('.team-a-checkbox, .team-b-checkbox').forEach(cb => {{
                    cb.addEventListener('change', updateCheckboxes);
                }});
            "#, MAX_PER_TEAM)))
        }
    };

    Html(base("Record Result", "record", content).into_string())
}

/// Submit match result (htmx endpoint)
pub async fn submit_result(
    State(state): State<Arc<AppState>>,
    Form(form): Form<RecordForm>,
) -> impl IntoResponse {
    let team_a_names = form.team_a.unwrap_or_default();
    let team_b_names = form.team_b.unwrap_or_default();

    // Validation
    if team_a_names.is_empty() || team_b_names.is_empty() {
        return Html(html! {
            p class="error" { "Both teams must have players" }
        }.into_string());
    }

    // Check for overlap
    let overlap: Vec<_> = team_a_names.iter().filter(|n| team_b_names.contains(n)).collect();
    if !overlap.is_empty() {
        let overlap_str = overlap.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ");
        return Html(html! {
            p class="error" { "Players cannot be on both teams: " (overlap_str) }
        }.into_string());
    }

    // Load players from database
    let all_players = match db::get_all_players(&state.db).await {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("Failed to load players: {}", e);
            return Html(html! {
                p class="error" { "Failed to load players" }
            }.into_string());
        }
    };

    let player_map: HashMap<&str, &Player> = all_players.iter().map(|p| (p.name.as_str(), p)).collect();

    let team_a: Vec<Player> = team_a_names
        .iter()
        .filter_map(|n| player_map.get(n.as_str()).map(|p| (*p).clone()))
        .collect();

    let team_b: Vec<Player> = team_b_names
        .iter()
        .filter_map(|n| player_map.get(n.as_str()).map(|p| (*p).clone()))
        .collect();

    if team_a.len() != team_a_names.len() || team_b.len() != team_b_names.len() {
        return Html(html! {
            p class="error" { "Some players not found in database" }
        }.into_string());
    }

    // Calculate Elo changes
    let elo_changes = calculate_elo_changes(&team_a, &team_b, form.score_a, form.score_b);

    // Build snapshot
    let snapshot: HashMap<String, EloSnapshot> = elo_changes.clone();
    let snapshot_json = serde_json::to_value(&snapshot).unwrap_or(json!({}));

    // Update player Elos in database
    for (name, change) in &elo_changes {
        let new_elo = change.before + change.delta;
        if let Err(e) = db::update_player_elo(&state.db, name, new_elo).await {
            tracing::error!("Failed to update Elo for {}: {}", name, e);
        }
    }

    // Save match record
    if let Err(e) = db::create_match(
        &state.db,
        &team_a_names,
        &team_b_names,
        form.score_a,
        form.score_b,
        snapshot_json,
    ).await {
        tracing::error!("Failed to save match: {}", e);
        return Html(html! {
            p class="error" { "Failed to save match record" }
        }.into_string());
    }

    // Render success with Elo changes
    Html(render_result(&team_a, &team_b, form.score_a, form.score_b, &elo_changes).into_string())
}

/// Form data for recording a match
#[derive(serde::Deserialize)]
pub struct RecordForm {
    team_a: Option<Vec<String>>,
    team_b: Option<Vec<String>>,
    score_a: i32,
    score_b: i32,
}

/// Render the match result with Elo changes
fn render_result(
    team_a: &[Player],
    team_b: &[Player],
    score_a: i32,
    score_b: i32,
    elo_changes: &HashMap<String, EloSnapshot>,
) -> Markup {
    let result_text = if score_a > score_b {
        "Team A wins!"
    } else if score_b > score_a {
        "Team B wins!"
    } else {
        "Draw!"
    };

    html! {
        article {
            header { "Match Recorded!" }

            h3 { (result_text) }
            p { "Score: " (score_a) " - " (score_b) }

            div class="team-grid" {
                // Team A changes
                div {
                    h4 { "Team A" }
                    ul class="player-list" {
                        @for player in team_a {
                            @if let Some(change) = elo_changes.get(&player.name) {
                                li {
                                    (player.name) ": "
                                    (render_elo_delta(change.delta))
                                    " (" (format!("{:.0}", change.before)) " → " (format!("{:.0}", change.before + change.delta)) ")"
                                }
                            }
                        }
                    }
                }

                // Team B changes
                div {
                    h4 { "Team B" }
                    ul class="player-list" {
                        @for player in team_b {
                            @if let Some(change) = elo_changes.get(&player.name) {
                                li {
                                    (player.name) ": "
                                    (render_elo_delta(change.delta))
                                    " (" (format!("{:.0}", change.before)) " → " (format!("{:.0}", change.before + change.delta)) ")"
                                }
                            }
                        }
                    }
                }
            }

            footer {
                a href="/history" { "View History →" }
            }
        }
    }
}
