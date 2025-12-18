use crate::balance::balance_teams;
use crate::elo::average_elo;
use crate::models::{TeamSplit, MAX_PLAYERS, TAG_WEIGHTS};
use crate::views::layout::{base, render_tags};
use crate::{db, AppState};
use axum::{
    extract::State,
    response::{Html, IntoResponse},
};
use axum_extra::extract::Form;
use maud::{html, Markup};
use std::sync::Arc;

/// Match Day page - check-in and team generation
pub async fn page(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let players = db::get_all_players(&state.db).await.unwrap_or_default();

    let content = html! {
        h2 { "Match Day" }

        // Check-in section
        h3 { "Player Check-In" }
        @if players.is_empty() {
            p { "No players in database. Add players in the Roster page." }
        } @else {
            form id="checkin-form" {
                p { "Select players for today's match (max " (MAX_PLAYERS) "):" }
                div class="checkbox-grid" {
                    @for player in &players {
                        label {
                            input
                                type="checkbox"
                                name="player_ids"
                                value=(player.id)
                                class="player-checkbox";
                            (player.name)
                        }
                    }
                }

                hr;

                // Team generation buttons
                div class="grid" {
                    button
                        type="submit"
                        hx-post="/api/generate"
                        hx-target="#teams-display"
                    {
                        "Generate Teams"
                    }
                    button
                        type="submit"
                        class="secondary"
                        hx-post="/api/shuffle"
                        hx-target="#teams-display"
                    {
                        "Shuffle (Re-roll)"
                    }
                }
            }
        }

        // Teams display area
        div id="teams-display" {
            p class="secondary" { "Select players and click 'Generate Teams'" }
        }

        // Script to enforce max players and enable/disable buttons
        script {
            (maud::PreEscaped(format!(r#"
                const maxPlayers = {};
                const buttons = document.querySelectorAll('#checkin-form button[type="submit"]');

                function updateState() {{
                    const checked = document.querySelectorAll('.player-checkbox:checked').length;

                    // Disable unchecked boxes when at max
                    if (checked >= maxPlayers) {{
                        document.querySelectorAll('.player-checkbox:not(:checked)').forEach(c => c.disabled = true);
                    }} else {{
                        document.querySelectorAll('.player-checkbox').forEach(c => c.disabled = false);
                    }}

                    // Only enable buttons when exactly max players selected
                    buttons.forEach(btn => btn.disabled = checked !== maxPlayers);
                }}

                // Initial state
                updateState();

                // Listen for changes
                document.querySelectorAll('.player-checkbox').forEach(cb => {{
                    cb.addEventListener('change', updateState);
                }});
            "#, MAX_PLAYERS)))
        }
    };

    Html(base("Match Day", "match_day", content).into_string())
}

/// Generate teams endpoint (htmx)
pub async fn generate_teams(
    State(state): State<Arc<AppState>>,
    Form(form): Form<GenerateForm>,
) -> impl IntoResponse {
    tracing::info!("Generate teams called with: {:?}", form.player_ids);

    let player_ids: Vec<i32> = form
        .player_ids
        .iter()
        .filter_map(|s| s.parse().ok())
        .collect();

    tracing::info!("Parsed player_ids: {:?}", player_ids);

    if player_ids.len() < 2 {
        return Html(html! {
            p class="error" { "Select at least 2 players" }
        }.into_string());
    }

    let players = match db::get_players_by_ids(&state.db, &player_ids).await {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("Failed to get players: {}", e);
            return Html(html! {
                p class="error" { "Failed to load players" }
            }.into_string());
        }
    };

    match balance_teams(&players, false) {
        Some(split) => Html(render_teams(&split).into_string()),
        None => Html(html! {
            p class="error" { "Could not generate teams" }
        }.into_string()),
    }
}

/// Shuffle teams endpoint (htmx)
pub async fn shuffle_teams(
    State(state): State<Arc<AppState>>,
    Form(form): Form<GenerateForm>,
) -> impl IntoResponse {
    let player_ids: Vec<i32> = form
        .player_ids
        .iter()
        .filter_map(|s| s.parse().ok())
        .collect();

    if player_ids.len() < 2 {
        return Html(html! {
            p class="error" { "Select at least 2 players" }
        }.into_string());
    }

    let players = match db::get_players_by_ids(&state.db, &player_ids).await {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("Failed to get players: {}", e);
            return Html(html! {
                p class="error" { "Failed to load players" }
            }.into_string());
        }
    };

    match balance_teams(&players, true) {
        Some(split) => Html(render_teams(&split).into_string()),
        None => Html(html! {
            p class="error" { "Could not generate teams" }
        }.into_string()),
    }
}

/// Form data for team generation
#[derive(Debug, serde::Deserialize)]
pub struct GenerateForm {
    #[serde(default)]
    player_ids: Vec<String>,
}

/// Render the generated teams
fn render_teams(split: &TeamSplit) -> Markup {
    let elo_a = average_elo(&split.team_a);
    let elo_b = average_elo(&split.team_b);

    html! {
        h3 { "Generated Teams" }

        div class="team-grid" {
            // Team A
            article {
                header { "Team A" }
                p { strong { "Avg Elo: " (format!("{:.0}", elo_a)) } }
                ul class="player-list" {
                    @for player in &split.team_a {
                        li {
                            (player.name) " (" (format!("{:.0}", player.elo)) ")"
                            (render_tags(&player.tags))
                        }
                    }
                }
            }

            // Team B
            article {
                header { "Team B" }
                p { strong { "Avg Elo: " (format!("{:.0}", elo_b)) } }
                ul class="player-list" {
                    @for player in &split.team_b {
                        li {
                            (player.name) " (" (format!("{:.0}", player.elo)) ")"
                            (render_tags(&player.tags))
                        }
                    }
                }
            }
        }

        // Balance details
        details {
            summary { "Balance Details" }
            p { "Total Cost: " (format!("{:.1}", split.cost)) }
            p { "Elo Difference: " (format!("{:.1}", split.elo_diff)) }
            @if !split.tag_costs.is_empty() {
                p { "Tag Imbalances:" }
                ul class="cost-breakdown" {
                    @for (tag, weight) in TAG_WEIGHTS {
                        @if let Some(&diff) = split.tag_costs.get(*tag) {
                            @if diff > 0 {
                                li { (tag) ": " (diff) " (penalty: " (diff * weight) ")" }
                            }
                        }
                    }
                }
            }
        }
    }
}
