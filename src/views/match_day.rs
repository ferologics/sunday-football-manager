use crate::auth::is_authenticated;
use crate::balance::balance_teams;
use crate::elo::average_elo;
use crate::models::{Player, Tag, TeamSplit};
use crate::views::layout::{base, render_tags, AuthState};
use crate::{db, AppState};
use axum::{
    extract::{Query, State},
    response::{Html, IntoResponse},
};
use axum_extra::extract::{cookie::CookieJar, Form};
use maud::{html, Markup, PreEscaped};
use serde::Deserialize;
use std::sync::Arc;

/// Team Generator page - check-in and team generation
pub async fn page(State(state): State<Arc<AppState>>, jar: CookieJar) -> impl IntoResponse {
    let players = db::get_all_players(&state.db).await.unwrap_or_default();
    let auth = AuthState::new(
        state.auth_password.is_some(),
        is_authenticated(&jar, &state),
    );

    let content = html! {
        h2 { "Team Generator" }

        // Check-in section
        h3 { "Player Check-In" }
        @if players.is_empty() {
            p { "No players in database. Add players in the Roster page." }
        } @else {
            form id="checkin-form" {
                p { "Select players for today's match: " span id="player-count" class="secondary" { "0 / 14" } }
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
                        hx-indicator="#generate-spinner"
                    {
                        "Generate Teams"
                        span id="generate-spinner" class="htmx-indicator spinner" {}
                    }
                    button
                        type="submit"
                        class="secondary"
                        hx-post="/api/shuffle"
                        hx-target="#teams-display"
                        hx-indicator="#shuffle-spinner"
                    {
                        "Shuffle (Re-roll)"
                        span id="shuffle-spinner" class="htmx-indicator spinner" {}
                    }
                }
            }
        }

        // Teams display area
        div id="teams-display" {
            p class="secondary" { "Select players and click 'Generate Teams'" }
        }

        // Script to enable/disable buttons and update counter
        script {
            (PreEscaped(r#"
                const MAX_PLAYERS = 14;
                const buttons = document.querySelectorAll('#checkin-form button[type="submit"]');
                const counter = document.getElementById('player-count');
                const checkboxes = document.querySelectorAll('.player-checkbox');

                function updateState() {
                    const checked = document.querySelectorAll('.player-checkbox:checked').length;
                    // Update counter
                    counter.textContent = checked + ' / ' + MAX_PLAYERS;
                    // Enable buttons when at least 2 players selected (minimum for teams)
                    buttons.forEach(btn => btn.disabled = checked < 2);
                    // Disable unchecked boxes when at max
                    checkboxes.forEach(cb => {
                        if (!cb.checked) cb.disabled = checked >= MAX_PLAYERS;
                    });
                }

                // Initial state
                updateState();

                // Listen for changes
                checkboxes.forEach(cb => {
                    cb.addEventListener('change', updateState);
                });

                // Parse comma-separated IDs (mirrors Rust parse_team_ids)
                function parseTeamIds(param) {
                    if (!param) return [];
                    return param.split(',').map(s => parseInt(s, 10)).filter(n => !isNaN(n));
                }

                // Encode team IDs to hash format (mirrors Rust encode_teams_hash)
                function encodeTeamsHash(teamA, teamB) {
                    return 'a=' + teamA.join(',') + '&b=' + teamB.join(',');
                }

                // Restore checkbox selection from team IDs
                function restoreCheckboxes(teamIds) {
                    checkboxes.forEach(cb => {
                        if (teamIds.includes(parseInt(cb.value))) {
                            cb.checked = true;
                        }
                    });
                    updateState();
                }

                // On page load: restore state from hash or localStorage
                window.addEventListener('load', () => {
                    const hash = window.location.hash.slice(1);
                    let teamIds = null;

                    if (hash && hash.includes('a=') && hash.includes('b=')) {
                        const params = new URLSearchParams(hash);
                        const teamA = parseTeamIds(params.get('a'));
                        const teamB = parseTeamIds(params.get('b'));
                        teamIds = [...teamA, ...teamB];
                        htmx.ajax('GET', '/api/teams?' + hash, '#teams-display');
                    } else {
                        const saved = localStorage.getItem('lastTeams');
                        if (saved) {
                            try {
                                const { teamA, teamB } = JSON.parse(saved);
                                teamIds = [...teamA, ...teamB];
                                const hash = encodeTeamsHash(teamA, teamB);
                                htmx.ajax('GET', '/api/teams?' + hash, '#teams-display');
                                history.replaceState(null, '', '#' + hash);
                            } catch (e) {}
                        }
                    }

                    if (teamIds) restoreCheckboxes(teamIds);
                });

                // After teams generated: update hash + localStorage
                document.body.addEventListener('htmx:afterSwap', (e) => {
                    if (e.detail.target.id === 'teams-display') {
                        const result = e.detail.target.querySelector('[data-team-a]');
                        if (result) {
                            const teamA = JSON.parse(result.dataset.teamA);
                            const teamB = JSON.parse(result.dataset.teamB);
                            history.replaceState(null, '', '#' + encodeTeamsHash(teamA, teamB));
                            localStorage.setItem('lastTeams', JSON.stringify({teamA, teamB}));
                        }
                    }
                });

                // Copy link to clipboard
                function copyTeamLink() {
                    navigator.clipboard.writeText(window.location.href).then(() => {
                        const btn = document.getElementById('copy-link-btn');
                        const orig = btn.textContent;
                        btn.textContent = 'Copied!';
                        setTimeout(() => btn.textContent = orig, 2000);
                    });
                }
                window.copyTeamLink = copyTeamLink;
            "#))
        }
    };

    Html(base("Team Generator", "match_day", &auth, content).into_string())
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
        return Html(
            html! {
                p class="error" { "Select at least 2 players" }
            }
            .into_string(),
        );
    }

    let players = match db::get_players_by_ids(&state.db, &player_ids).await {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("Failed to get players: {}", e);
            return Html(
                html! {
                    p class="error" { "Failed to load players" }
                }
                .into_string(),
            );
        }
    };

    match balance_teams(&players, false) {
        Some(split) => Html(render_teams(&split).into_string()),
        None => Html(
            html! {
                p class="error" { "Could not generate teams" }
            }
            .into_string(),
        ),
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
        return Html(
            html! {
                p class="error" { "Select at least 2 players" }
            }
            .into_string(),
        );
    }

    let players = match db::get_players_by_ids(&state.db, &player_ids).await {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("Failed to get players: {}", e);
            return Html(
                html! {
                    p class="error" { "Failed to load players" }
                }
                .into_string(),
            );
        }
    };

    match balance_teams(&players, true) {
        Some(split) => Html(render_teams(&split).into_string()),
        None => Html(
            html! {
                p class="error" { "Could not generate teams" }
            }
            .into_string(),
        ),
    }
}

/// Parse comma-separated IDs from URL param (e.g., "1,5,7" → [1, 5, 7])
fn parse_team_ids(param: &str) -> Vec<i32> {
    param.split(',').filter_map(|s| s.parse().ok()).collect()
}

/// Encode team IDs to URL hash format (e.g., [1,5,7], [2,3,6] → "a=1,5,7&b=2,3,6")
#[cfg(test)]
fn encode_teams_hash(team_a: &[i32], team_b: &[i32]) -> String {
    let join = |ids: &[i32]| {
        ids.iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(",")
    };
    format!("a={}&b={}", join(team_a), join(team_b))
}

/// View teams from URL params (for shareable links)
pub async fn view_teams(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ViewTeamsParams>,
) -> impl IntoResponse {
    let team_a_ids = parse_team_ids(&params.a);
    let team_b_ids = parse_team_ids(&params.b);

    if team_a_ids.is_empty() || team_b_ids.is_empty() {
        return Html(
            html! {
                p class="error" { "Invalid team data" }
            }
            .into_string(),
        );
    }

    // Fetch players for each team
    let team_a = match db::get_players_by_ids(&state.db, &team_a_ids).await {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("Failed to get team A players: {}", e);
            return Html(
                html! {
                    p class="error" { "Failed to load team A" }
                }
                .into_string(),
            );
        }
    };

    let team_b = match db::get_players_by_ids(&state.db, &team_b_ids).await {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("Failed to get team B players: {}", e);
            return Html(
                html! {
                    p class="error" { "Failed to load team B" }
                }
                .into_string(),
            );
        }
    };

    // Build a TeamSplit (cost/diff don't matter for display)
    let split = TeamSplit {
        team_a,
        team_b,
        cost: 0.0,
        elo_diff: 0.0,
        tag_value_a: 0,
        tag_value_b: 0,
    };

    Html(render_teams(&split).into_string())
}

/// Query params for viewing pre-defined teams
#[derive(Deserialize)]
pub struct ViewTeamsParams {
    a: String,
    b: String,
}

/// Form data for team generation
#[derive(Debug, Deserialize)]
pub struct GenerateForm {
    #[serde(default)]
    player_ids: Vec<String>,
}

/// Render the generated teams
fn render_teams(split: &TeamSplit) -> Markup {
    let elo_a = average_elo(&split.team_a);
    let elo_b = average_elo(&split.team_b);

    // Sort teams by Elo ascending and check for dedicated GKs
    let (team_a_sorted, team_a_has_gk) = sort_team_for_goal_rotation(&split.team_a);
    let (team_b_sorted, team_b_has_gk) = sort_team_for_goal_rotation(&split.team_b);

    // Serialize team IDs for JavaScript (sorted order)
    let team_a_ids: Vec<i32> = team_a_sorted.iter().map(|p| p.id).collect();
    let team_b_ids: Vec<i32> = team_b_sorted.iter().map(|p| p.id).collect();
    let team_a_json = serde_json::to_string(&team_a_ids).unwrap_or_else(|_| "[]".to_string());
    let team_b_json = serde_json::to_string(&team_b_ids).unwrap_or_else(|_| "[]".to_string());

    html! {
        // Data attributes for JS to read team IDs
        div data-team-a=(team_a_json) data-team-b=(team_b_json) {
            h3 { "Generated Teams" }

            div class="team-grid" {
                // Team A
                article {
                    header { "Team A" }
                    p { strong { "Avg Elo: " (format!("{:.0}", elo_a)) } }
                    @if team_a_has_gk {
                        ul class="player-list" style="padding-left: 1.25em;" {
                            @for player in &team_a_sorted {
                                li {
                                    (player.name) " (" (format!("{:.0}", player.elo)) ")"
                                    (render_tags(&player.tags))
                                }
                            }
                        }
                    } @else {
                        p class="secondary" style="font-size: 0.85em; margin-bottom: 0.5em;" { "🧤 Goal rotation order" }
                        ol class="player-list" style="padding-left: 1.5em;" {
                            @for player in &team_a_sorted {
                                li {
                                    (player.name) " (" (format!("{:.0}", player.elo)) ")"
                                    (render_tags(&player.tags))
                                }
                            }
                        }
                    }
                }

                // Team B
                article {
                    header { "Team B" }
                    p { strong { "Avg Elo: " (format!("{:.0}", elo_b)) } }
                    @if team_b_has_gk {
                        ul class="player-list" style="padding-left: 1.25em;" {
                            @for player in &team_b_sorted {
                                li {
                                    (player.name) " (" (format!("{:.0}", player.elo)) ")"
                                    (render_tags(&player.tags))
                                }
                            }
                        }
                    } @else {
                        p class="secondary" style="font-size: 0.85em; margin-bottom: 0.5em;" { "🧤 Goal rotation order" }
                        ol class="player-list" style="padding-left: 1.5em;" {
                            @for player in &team_b_sorted {
                                li {
                                    (player.name) " (" (format!("{:.0}", player.elo)) ")"
                                    (render_tags(&player.tags))
                                }
                            }
                        }
                    }
                }
            }

            // Balance details
            details {
                summary { "Balance Details" }
                p { "Elo Diff: " (format!("{:.1}", split.elo_diff)) }
                p {
                    "Tag Value: "
                    (split.tag_value_a) " vs " (split.tag_value_b)
                    " (diff: " ((split.tag_value_a - split.tag_value_b).abs()) ")"
                }
                p class="secondary" { "Total Cost: " (format!("{:.1}", split.cost)) }
            }

            // Action buttons
            div class="grid" style="margin-top: 1rem;" {
                button id="copy-link-btn" type="button" class="secondary outline" onclick="copyTeamLink()" {
                    "📋 Copy link"
                }
                button type="button" onclick="window.location.href='/record'" {
                    "Record this match →"
                }
            }
        }
    }
}

/// Sort team by Elo ascending for goal rotation order.
/// Returns (sorted_players, has_dedicated_gk).
fn sort_team_for_goal_rotation(team: &[Player]) -> (Vec<Player>, bool) {
    let mut sorted = team.to_vec();
    sorted.sort_by(|a, b| a.elo.partial_cmp(&b.elo).unwrap());
    let has_gk = sorted.iter().any(|p| p.has_tag(Tag::Gk));
    (sorted, has_gk)
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
    fn test_sort_team_by_elo_ascending() {
        let team = vec![
            make_player(1, "High", 1400.0, ""),
            make_player(2, "Low", 1000.0, ""),
            make_player(3, "Mid", 1200.0, ""),
        ];

        let (sorted, has_gk) = sort_team_for_goal_rotation(&team);

        assert!(!has_gk);
        assert_eq!(sorted[0].name, "Low");
        assert_eq!(sorted[1].name, "Mid");
        assert_eq!(sorted[2].name, "High");
    }

    #[test]
    fn test_team_with_gk_detected() {
        let team = vec![
            make_player(1, "Keeper", 1200.0, "GK"),
            make_player(2, "Player", 1200.0, ""),
        ];

        let (_, has_gk) = sort_team_for_goal_rotation(&team);
        assert!(has_gk);
    }

    #[test]
    fn test_team_without_gk() {
        let team = vec![
            make_player(1, "Runner", 1200.0, "RUNNER"),
            make_player(2, "Playmaker", 1200.0, "PLAYMAKER"),
        ];

        let (_, has_gk) = sort_team_for_goal_rotation(&team);
        assert!(!has_gk);
    }

    #[test]
    fn test_higher_elo_goes_last() {
        let team = vec![
            make_player(1, "Star", 1500.0, "PLAYMAKER"),
            make_player(2, "Newbie", 1000.0, ""),
            make_player(3, "Average", 1200.0, ""),
        ];

        let (sorted, _) = sort_team_for_goal_rotation(&team);

        assert_eq!(sorted[0].name, "Newbie"); // First in goal
        assert_eq!(sorted[2].name, "Star"); // Last in goal
    }

    #[test]
    fn test_parse_team_ids() {
        assert_eq!(parse_team_ids("1,5,7"), vec![1, 5, 7]);
        assert_eq!(parse_team_ids("42"), vec![42]);
        assert_eq!(parse_team_ids(""), Vec::<i32>::new());
        assert_eq!(parse_team_ids("1,invalid,3"), vec![1, 3]); // skips invalid
    }

    #[test]
    fn test_encode_teams_hash() {
        assert_eq!(encode_teams_hash(&[1, 5, 7], &[2, 3, 6]), "a=1,5,7&b=2,3,6");
        assert_eq!(encode_teams_hash(&[42], &[13]), "a=42&b=13");
    }

    #[test]
    fn test_team_ids_roundtrip() {
        let team_a = vec![1, 5, 7];
        let team_b = vec![2, 3, 6];

        let hash = encode_teams_hash(&team_a, &team_b);
        // Parse like view_teams does from query params
        let params: std::collections::HashMap<&str, &str> =
            hash.split('&').filter_map(|p| p.split_once('=')).collect();

        let parsed_a = parse_team_ids(params["a"]);
        let parsed_b = parse_team_ids(params["b"]);

        assert_eq!(parsed_a, team_a);
        assert_eq!(parsed_b, team_b);
    }
}
