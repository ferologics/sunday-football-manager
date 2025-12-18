use crate::auth::is_authenticated;
use crate::elo::calculate_elo_changes;
use crate::models::{EloSnapshot, Player, MAX_PER_TEAM};
use crate::views::layout::{base, render_elo_delta, AuthState};
use crate::{db, AppState};
use axum::{
    extract::State,
    response::{Html, IntoResponse},
};
use axum_extra::extract::{cookie::CookieJar, Form};
use maud::{html, Markup};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

/// Record Result page
pub async fn page(State(state): State<Arc<AppState>>, jar: CookieJar) -> impl IntoResponse {
    let players = db::get_all_players(&state.db).await.unwrap_or_default();
    let logged_in = is_authenticated(&jar, &state);
    let auth = AuthState::new(state.auth_password.is_some(), logged_in);

    // Serialize players for JavaScript
    let players_json: Vec<serde_json::Value> = players
        .iter()
        .map(|p| json!({ "name": p.name, "elo": p.elo }))
        .collect();
    let players_json_str = serde_json::to_string(&players_json).unwrap_or_else(|_| "[]".to_string());

    let content = html! {
        h2 { "Record Match Result" }

        // CSS for chip selector
        style {
            (maud::PreEscaped(r#"
                .player-select { position: relative; }
                .player-search { width: 100%; margin-bottom: 0.5rem; }
                .player-dropdown {
                    position: absolute;
                    z-index: 100;
                    width: 100%;
                    max-height: 200px;
                    overflow-y: auto;
                    background: var(--pico-card-background-color);
                    border: 1px solid var(--pico-muted-border-color);
                    border-radius: var(--pico-border-radius);
                    list-style: none;
                    margin: 0;
                    padding: 0;
                    display: none;
                }
                .player-dropdown.open { display: block; }
                .player-dropdown li {
                    padding: 0.5rem 0.75rem;
                    cursor: pointer;
                }
                .player-dropdown li:hover {
                    background: var(--pico-primary-hover-background);
                }
                .selected-chips {
                    display: flex;
                    flex-wrap: wrap;
                    gap: 0.5rem;
                    min-height: 2.5rem;
                    margin-top: 0.5rem;
                }
                .chip {
                    display: inline-flex;
                    align-items: center;
                    gap: 0.25rem;
                    padding: 0.25rem 0.5rem;
                    background: var(--pico-primary-background);
                    color: var(--pico-primary-inverse);
                    border-radius: 1rem;
                    font-size: 0.875rem;
                }
                .chip button {
                    background: none;
                    border: none;
                    color: inherit;
                    cursor: pointer;
                    padding: 0 0.25rem;
                    margin: 0;
                    font-size: 1rem;
                    line-height: 1;
                }
                .chip button:hover { opacity: 0.7; }
            "#))
        }

        form id="record-form" hx-post="/api/record" hx-target="#result-display" {
            // Team selection
            div class="team-grid" {
                // Team A
                fieldset {
                    legend { "Team A (max " (MAX_PER_TEAM) ")" }
                    div class="player-select" data-team="a" {
                        input
                            type="text"
                            class="player-search"
                            placeholder="Search players..."
                            autocomplete="off";
                        ul class="player-dropdown" {}
                        div class="selected-chips" {}
                    }
                }

                // Team B
                fieldset {
                    legend { "Team B (max " (MAX_PER_TEAM) ")" }
                    div class="player-select" data-team="b" {
                        input
                            type="text"
                            class="player-search"
                            placeholder="Search players..."
                            autocomplete="off";
                        ul class="player-dropdown" {}
                        div class="selected-chips" {}
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

            button type="submit" disabled[!logged_in] { "Submit Result" }
            @if !logged_in {
                p class="secondary" style="margin-top: 0.5rem; font-size: 0.875rem;" { "Login to record results" }
            }
        }

        // Result display area
        div id="result-display" {}

        // JavaScript for chip selector
        script {
            (maud::PreEscaped(format!(r#"
                const allPlayers = {players_json};
                const maxPerTeam = {max_per_team};
                const selectedA = new Set();
                const selectedB = new Set();

                function getAvailable() {{
                    return allPlayers.filter(p => !selectedA.has(p.name) && !selectedB.has(p.name));
                }}

                function renderDropdown(container, filter) {{
                    const dropdown = container.querySelector('.player-dropdown');
                    const team = container.dataset.team;
                    const selected = team === 'a' ? selectedA : selectedB;

                    if (selected.size >= maxPerTeam) {{
                        dropdown.innerHTML = '<li style="color: var(--pico-muted-color)">Max players reached</li>';
                        return;
                    }}

                    const available = getAvailable();
                    const filtered = filter
                        ? available.filter(p => p.name.toLowerCase().includes(filter.toLowerCase()))
                        : available;

                    if (filtered.length === 0) {{
                        dropdown.innerHTML = '<li style="color: var(--pico-muted-color)">No players found</li>';
                        return;
                    }}

                    dropdown.innerHTML = filtered.map(p =>
                        `<li data-name="${{p.name}}">${{p.name}}</li>`
                    ).join('');

                    dropdown.querySelectorAll('li[data-name]').forEach(li => {{
                        li.addEventListener('click', () => {{
                            selectPlayer(container, li.dataset.name);
                        }});
                    }});
                }}

                function selectPlayer(container, name) {{
                    const team = container.dataset.team;
                    const selected = team === 'a' ? selectedA : selectedB;
                    const inputName = team === 'a' ? 'team_a' : 'team_b';

                    if (selected.size >= maxPerTeam) return;

                    selected.add(name);

                    // Add chip
                    const chipsContainer = container.querySelector('.selected-chips');
                    const chip = document.createElement('span');
                    chip.className = 'chip';
                    chip.dataset.name = name;
                    chip.innerHTML = `${{name}} <button type="button">&times;</button>`;
                    chip.querySelector('button').addEventListener('click', () => {{
                        removePlayer(container, name);
                    }});
                    chipsContainer.appendChild(chip);

                    // Add hidden input for form
                    const hidden = document.createElement('input');
                    hidden.type = 'hidden';
                    hidden.name = inputName;
                    hidden.value = name;
                    hidden.dataset.playerName = name;
                    container.appendChild(hidden);

                    // Clear search and close dropdown
                    const search = container.querySelector('.player-search');
                    search.value = '';
                    container.querySelector('.player-dropdown').classList.remove('open');
                }}

                function removePlayer(container, name) {{
                    const team = container.dataset.team;
                    const selected = team === 'a' ? selectedA : selectedB;

                    selected.delete(name);

                    // Remove chip
                    const chip = container.querySelector(`.chip[data-name="${{name}}"]`);
                    if (chip) chip.remove();

                    // Remove hidden input
                    const hidden = container.querySelector(`input[data-player-name="${{name}}"]`);
                    if (hidden) hidden.remove();
                }}

                // Setup event listeners
                document.querySelectorAll('.player-select').forEach(container => {{
                    const search = container.querySelector('.player-search');
                    const dropdown = container.querySelector('.player-dropdown');

                    search.addEventListener('focus', () => {{
                        renderDropdown(container, search.value);
                        dropdown.classList.add('open');
                    }});

                    search.addEventListener('input', () => {{
                        renderDropdown(container, search.value);
                        dropdown.classList.add('open');
                    }});
                }});

                // Close dropdown when clicking outside
                document.addEventListener('click', (e) => {{
                    document.querySelectorAll('.player-select').forEach(container => {{
                        if (!container.contains(e.target)) {{
                            container.querySelector('.player-dropdown').classList.remove('open');
                        }}
                    }});
                }});
            "#, players_json = players_json_str, max_per_team = MAX_PER_TEAM)))
        }
    };

    Html(base("Record Result", "record", &auth, content).into_string())
}

/// Submit match result (htmx endpoint)
pub async fn submit_result(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    Form(form): Form<RecordForm>,
) -> impl IntoResponse {
    if !is_authenticated(&jar, &state) {
        return Html(html! {
            p class="error" { "Unauthorized. Please log in." }
        }.into_string());
    }

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
