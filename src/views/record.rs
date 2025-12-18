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

    // Serialize players for JavaScript (include ID for participation tracking)
    let players_json: Vec<serde_json::Value> = players
        .iter()
        .map(|p| json!({ "id": p.id, "name": p.name, "elo": p.elo }))
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
                .participation-row {
                    display: flex;
                    justify-content: space-between;
                    align-items: center;
                    padding: 0.5rem;
                    border-bottom: 1px solid var(--pico-muted-border-color);
                }
                .participation-row:last-child { border-bottom: none; }
                .participation-row.partial {
                    background: var(--pico-del-color);
                    background: color-mix(in srgb, var(--pico-del-color) 15%, transparent);
                }
                .participation-row select {
                    width: auto;
                    margin: 0;
                    padding: 0.25rem 0.5rem;
                }
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

            // Participation section (collapsed by default)
            details id="participation-section" {
                summary { "Participation (expand if someone played partial)" }
                div id="participation-list" {
                    p class="secondary" { "Select players first" }
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

            button type="submit" disabled[!logged_in] hx-indicator="#submit-spinner" {
                "Submit Result"
                span id="submit-spinner" class="htmx-indicator spinner" {}
            }
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
                        `<li data-name="${{p.name}}" data-id="${{p.id}}">${{p.name}}</li>`
                    ).join('');

                    dropdown.querySelectorAll('li[data-name]').forEach(li => {{
                        li.addEventListener('click', () => {{
                            selectPlayer(container, li.dataset.name, parseInt(li.dataset.id));
                        }});
                    }});
                }}

                // Track participation values (default 1.0)
                const participationValues = {{}};

                function selectPlayer(container, name, playerId) {{
                    const team = container.dataset.team;
                    const selected = team === 'a' ? selectedA : selectedB;
                    const inputName = team === 'a' ? 'team_a' : 'team_b';

                    if (selected.size >= maxPerTeam) return;

                    selected.add(name);
                    participationValues[playerId] = 1.0;

                    // Add simple chip (just name and remove button)
                    const chipsContainer = container.querySelector('.selected-chips');
                    const chip = document.createElement('span');
                    chip.className = 'chip';
                    chip.dataset.name = name;
                    chip.dataset.playerId = playerId;
                    chip.innerHTML = `${{name}}<button type="button">&times;</button>`;

                    chip.querySelector('button').addEventListener('click', () => {{
                        removePlayer(container, name, playerId);
                    }});
                    chipsContainer.appendChild(chip);

                    // Add hidden input for team
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

                    // Update participation list
                    renderParticipationList();
                }}

                function updateParticipation(playerId, value) {{
                    participationValues[playerId] = parseFloat(value);
                    // Update row styling and hidden input
                    const row = document.querySelector(`.participation-row[data-player-id="${{playerId}}"]`);
                    if (row) {{
                        row.classList.toggle('partial', value !== '1.0');
                        const hidden = row.querySelector('input[name="participation"]');
                        if (hidden) hidden.value = `${{playerId}}=${{value}}`;
                    }}
                }}

                function removePlayer(container, name, playerId) {{
                    const team = container.dataset.team;
                    const selected = team === 'a' ? selectedA : selectedB;

                    selected.delete(name);
                    delete participationValues[playerId];

                    // Remove chip
                    const chip = container.querySelector(`.chip[data-name="${{name}}"]`);
                    if (chip) chip.remove();

                    // Remove hidden input
                    const hidden = container.querySelector(`input[data-player-name="${{name}}"]`);
                    if (hidden) hidden.remove();

                    // Update participation list
                    renderParticipationList();
                }}

                function renderParticipationList() {{
                    const list = document.getElementById('participation-list');
                    const allSelected = [];

                    // Gather all selected players
                    document.querySelectorAll('.chip').forEach(chip => {{
                        const playerId = parseInt(chip.dataset.playerId);
                        const name = chip.dataset.name;
                        const team = chip.closest('.player-select').dataset.team;
                        allSelected.push({{ playerId, name, team }});
                    }});

                    if (allSelected.length === 0) {{
                        list.innerHTML = '<p class="secondary">Select players first</p>';
                        return;
                    }}

                    // Build participation list HTML
                    let html = '';
                    allSelected.forEach(({{ playerId, name, team }}) => {{
                        const value = participationValues[playerId] || 1.0;
                        const isPartial = value < 1.0;
                        html += `
                            <div class="participation-row${{isPartial ? ' partial' : ''}}" data-player-id="${{playerId}}">
                                <span>${{name}} <small class="secondary">(Team ${{team.toUpperCase()}})</small></span>
                                <select onchange="updateParticipation(${{playerId}}, this.value)">
                                    <option value="1.0"${{value === 1.0 ? ' selected' : ''}}>100%</option>
                                    <option value="0.75"${{value === 0.75 ? ' selected' : ''}}>75%</option>
                                    <option value="0.5"${{value === 0.5 ? ' selected' : ''}}>50%</option>
                                    <option value="0.25"${{value === 0.25 ? ' selected' : ''}}>25%</option>
                                </select>
                                <input type="hidden" name="participation" value="${{playerId}}=${{value}}">
                            </div>
                        `;
                    }});
                    list.innerHTML = html;
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

    // Validate scores (0-50 range)
    let score_a = form.score_a.clamp(0, 50);
    let score_b = form.score_b.clamp(0, 50);

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

    // Warn about uneven teams (soft check with confirmation)
    if team_a.len() != team_b.len() && !form.confirm_uneven {
        return Html(html! {
            article {
                header { "Uneven Teams" }
                p {
                    "Team A has " (team_a.len()) " players, Team B has " (team_b.len()) " players."
                }
                p { "Are you sure you want to record this match?" }
                button
                    hx-post="/api/record"
                    hx-include="closest form"
                    hx-vals=r#"{"confirm_uneven": true}"#
                    hx-target="#result-display"
                {
                    "Yes, record match"
                }
            }
        }.into_string());
    }

    // Build participation map from form data (format: "PlayerID=0.75")
    let mut participation: HashMap<i32, f32> = HashMap::new();
    if let Some(parts) = &form.participation {
        for entry in parts {
            if let Some((id_str, value)) = entry.split_once('=') {
                if let (Ok(id), Ok(v)) = (id_str.parse::<i32>(), value.parse::<f32>()) {
                    participation.insert(id, v.clamp(0.0, 1.0));
                }
            }
        }
    }

    // Calculate Elo changes with handicap system (keyed by player ID)
    let elo_changes = calculate_elo_changes(&team_a, &team_b, score_a, score_b, &participation);

    // Build snapshot (keyed by player ID)
    let snapshot: HashMap<i32, EloSnapshot> = elo_changes.clone();
    let snapshot_json = serde_json::to_value(&snapshot).unwrap_or(json!({}));

    // Use a transaction to ensure all updates are atomic
    let mut tx = match state.db.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!("Failed to start transaction: {}", e);
            return Html(html! {
                p class="error" { "Database error" }
            }.into_string());
        }
    };

    // Update player Elos in database (applying participation for partial credit)
    for player in team_a.iter().chain(team_b.iter()) {
        if let Some(change) = elo_changes.get(&player.id) {
            // Apply participation: injured players get proportional Elo change
            let effective_delta = change.delta * change.participation;
            let new_elo = change.before + effective_delta;
            if let Err(e) = db::update_player_elo(&mut *tx, player.id, new_elo).await {
                tracing::error!("Failed to update Elo for {}: {}", player.name, e);
                return Html(html! {
                    p class="error" { "Failed to update player Elo" }
                }.into_string());
            }
        }
    }

    // Save match record (with player IDs)
    let team_a_ids: Vec<i32> = team_a.iter().map(|p| p.id).collect();
    let team_b_ids: Vec<i32> = team_b.iter().map(|p| p.id).collect();
    if let Err(e) = db::create_match(
        &mut *tx,
        &team_a_ids,
        &team_b_ids,
        score_a,
        score_b,
        snapshot_json,
    ).await {
        tracing::error!("Failed to save match: {}", e);
        return Html(html! {
            p class="error" { "Failed to save match record" }
        }.into_string());
    }

    // Commit the transaction
    if let Err(e) = tx.commit().await {
        tracing::error!("Failed to commit transaction: {}", e);
        return Html(html! {
            p class="error" { "Failed to save changes" }
        }.into_string());
    }

    // Render success with Elo changes
    Html(render_result(&team_a, &team_b, score_a, score_b, &elo_changes).into_string())
}

/// Form data for recording a match
#[derive(serde::Deserialize)]
pub struct RecordForm {
    team_a: Option<Vec<String>>,
    team_b: Option<Vec<String>>,
    score_a: i32,
    score_b: i32,
    #[serde(default)]
    confirm_uneven: bool,
    /// Participation percentages: "PlayerName=0.75" format
    #[serde(default)]
    participation: Option<Vec<String>>,
}

/// Render the match result with Elo changes
fn render_result(
    team_a: &[Player],
    team_b: &[Player],
    score_a: i32,
    score_b: i32,
    elo_changes: &HashMap<i32, EloSnapshot>,
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
                            @if let Some(change) = elo_changes.get(&player.id) {
                                @let effective_delta = change.delta * change.participation;
                                li {
                                    (player.name) ": "
                                    (render_elo_delta(effective_delta))
                                    @if change.participation < 1.0 {
                                        span class="secondary" style="font-size: 0.8em;" {
                                            " (" (format!("{:.0}%", change.participation * 100.0)) ")"
                                        }
                                    }
                                    " (" (format!("{:.0}", change.before)) " → " (format!("{:.0}", change.before + effective_delta)) ")"
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
                            @if let Some(change) = elo_changes.get(&player.id) {
                                @let effective_delta = change.delta * change.participation;
                                li {
                                    (player.name) ": "
                                    (render_elo_delta(effective_delta))
                                    @if change.participation < 1.0 {
                                        span class="secondary" style="font-size: 0.8em;" {
                                            " (" (format!("{:.0}%", change.participation * 100.0)) ")"
                                        }
                                    }
                                    " (" (format!("{:.0}", change.before)) " → " (format!("{:.0}", change.before + effective_delta)) ")"
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
