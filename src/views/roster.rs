use crate::auth::is_authenticated;
use crate::models::{NewPlayer, UpdatePlayer, TAG_WEIGHTS};
use crate::views::layout::{base, render_tags, AuthState};
use crate::{db, AppState};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse},
};
use axum_extra::extract::{cookie::CookieJar, Form};
use maud::{html, Markup};
use std::sync::Arc;

/// Roster page - player management
pub async fn page(State(state): State<Arc<AppState>>, jar: CookieJar) -> impl IntoResponse {
    let players = db::get_all_players(&state.db).await.unwrap_or_default();
    let logged_in = is_authenticated(&jar, &state);
    let auth = AuthState::new(state.auth_password.is_some(), logged_in);

    let content = html! {
        h2 { "Roster Management" }

        // Add player form
        details open {
            summary { "Add New Player" }
            form hx-post="/api/players" hx-target="#player-list" hx-swap="innerHTML" hx-on--after-request="if(event.detail.successful) this.reset()" {
                div class="grid" {
                    input type="text" name="name" placeholder="Player name" required disabled[!logged_in];
                    input type="number" name="elo" placeholder="Starting Elo" value="1200" min="800" max="2000" disabled[!logged_in];
                }
                fieldset {
                    legend { "Tags" }
                    div class="checkbox-grid" {
                        @for (tag, _) in TAG_WEIGHTS {
                            label {
                                input type="checkbox" name="tags" value=(tag) disabled[!logged_in];
                                (tag)
                            }
                        }
                        label {
                            input type="checkbox" name="tags" value="GK" disabled[!logged_in];
                            "GK"
                        }
                    }
                }
                button type="submit" disabled[!logged_in] hx-indicator="#add-spinner" {
                    "Add Player"
                    span id="add-spinner" class="htmx-indicator spinner" {}
                }
                @if !logged_in {
                    p class="secondary login-hint" { "Login to add players" }
                }
            }
        }

        hr;

        // Player list
        h3 { "Current Roster (" (players.len()) " players)" }
        div id="player-list" {
            (render_player_list(&players, logged_in))
        }
    };

    Html(base("Roster", "roster", &auth, content).into_string())
}

/// Render the player list (used for full page and htmx updates)
fn render_player_list(players: &[crate::models::Player], logged_in: bool) -> Markup {
    if players.is_empty() {
        return html! {
            p { "No players yet. Add your first player above!" }
        };
    }

    html! {
        div class="table-container" {
            table {
                thead {
                    tr {
                        th { "Name" }
                        th { "Elo" }
                        th { "Tags" }
                        th { "Matches" }
                        th { "Actions" }
                    }
                }
                tbody {
                    @for player in players {
                        tr id=(format!("player-{}", player.id)) {
                            td { (player.name) }
                            td { (format!("{:.0}", player.elo)) }
                            td { (render_tags(&player.tags)) }
                            td { (player.matches_played) }
                            td {
                                button
                                    class="secondary outline"
                                    hx-delete=(format!("/api/players/{}", player.id))
                                    hx-target="#player-list"
                                    hx-swap="innerHTML"
                                    hx-confirm=(format!("Delete {}?", player.name))
                                    disabled[!logged_in]
                                {
                                    "Delete"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Create a new player (htmx endpoint)
pub async fn create_player(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    Form(form): Form<NewPlayerForm>,
) -> impl IntoResponse {
    if !is_authenticated(&jar, &state) {
        return crate::auth::unauthorized().into_response();
    }

    // Combine tags from checkboxes
    let tags = form.tags.unwrap_or_default().join(",");

    let new_player = NewPlayer {
        name: form.name,
        elo: form.elo,
        tags: Some(tags),
    };

    match db::create_player(&state.db, &new_player).await {
        Ok(player) => {
            let players = db::get_all_players(&state.db).await.unwrap_or_default();
            Html(
                html! {
                    p class="success-message" { "Added " (player.name) "!" }
                    (render_player_list(&players, true))
                }
                .into_string(),
            )
            .into_response()
        }
        Err(e) => {
            tracing::error!("Failed to create player: {}", e);
            Html(
                html! {
                    p class="error" { "Failed to create player: name may already exist" }
                }
                .into_string(),
            )
            .into_response()
        }
    }
}

/// Form data for creating a player (with multiple tags as checkboxes)
#[derive(serde::Deserialize)]
pub struct NewPlayerForm {
    name: String,
    elo: Option<f32>,
    tags: Option<Vec<String>>,
}

/// Update a player (htmx endpoint)
pub async fn update_player(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    Path(id): Path<i32>,
    Form(form): Form<UpdatePlayer>,
) -> impl IntoResponse {
    if !is_authenticated(&jar, &state) {
        return crate::auth::unauthorized().into_response();
    }

    match db::update_player(&state.db, id, &form).await {
        Ok(Some(_)) => {
            let players = db::get_all_players(&state.db).await.unwrap_or_default();
            Html(render_player_list(&players, true).into_string()).into_response()
        }
        Ok(None) => (StatusCode::NOT_FOUND, "Player not found").into_response(),
        Err(e) => {
            tracing::error!("Failed to update player: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to update player").into_response()
        }
    }
}

/// Delete a player (htmx endpoint)
pub async fn delete_player(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    Path(id): Path<i32>,
) -> impl IntoResponse {
    if !is_authenticated(&jar, &state) {
        return crate::auth::unauthorized().into_response();
    }

    match db::delete_player(&state.db, id).await {
        Ok(true) => {
            let players = db::get_all_players(&state.db).await.unwrap_or_default();
            Html(render_player_list(&players, true).into_string()).into_response()
        }
        Ok(false) => (StatusCode::NOT_FOUND, "Player not found").into_response(),
        Err(e) => {
            tracing::error!("Failed to delete player: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to delete player").into_response()
        }
    }
}
