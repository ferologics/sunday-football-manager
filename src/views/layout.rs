use maud::{html, Markup, DOCTYPE};

/// Auth state for layout
pub struct AuthState {
    pub enabled: bool,
    pub logged_in: bool,
}

impl AuthState {
    pub fn new(enabled: bool, logged_in: bool) -> Self {
        Self { enabled, logged_in }
    }
}

/// Base layout wrapper for all pages
pub fn base(title: &str, current_page: &str, auth: &AuthState, content: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (title) " - Sunday Manager" }
                // PicoCSS
                link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/@picocss/pico@2/css/pico.min.css";
                // htmx
                script src="https://unpkg.com/htmx.org@2.0.4" {}
                // Custom styles
                style {
                    r#"
                    .nav-buttons { display: flex; gap: 0.5rem; flex-wrap: wrap; margin-bottom: 1rem; }
                    .nav-buttons a { flex: 1; text-align: center; min-width: 120px; }
                    .team-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 1rem; }
                    .player-list { list-style: none; padding: 0; }
                    .player-list li { padding: 0.5rem; border-bottom: 1px solid var(--pico-muted-border-color); }
                    .tag { display: inline-block; padding: 0.1rem 0.4rem; border-radius: 4px; font-size: 0.75rem; background: var(--pico-primary-background); color: var(--pico-primary-inverse); margin-left: 0.25rem; }
                    .elo-positive { color: var(--pico-ins-color); }
                    .elo-negative { color: var(--pico-del-color); }
                    .cost-breakdown { font-size: 0.875rem; color: var(--pico-muted-color); }
                    .checkbox-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap: 0.5rem; }
                    .header-row { display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 1rem; margin-bottom: 1rem; }
                    .auth-form { display: flex; gap: 0.5rem; align-items: stretch; margin: 0; }
                    .auth-form input, .auth-form button { margin: 0; padding: 0.5rem 0.75rem; height: auto; }
                    .auth-form input { width: 150px; }
                    .auth-status { display: flex; gap: 0.5rem; align-items: center; }
                    "#
                }
            }
            body {
                main class="container" {
                    // Header with title and auth
                    div class="header-row" {
                        h1 style="margin: 0;" { "Sunday Football Manager" }

                        @if auth.enabled {
                            @if auth.logged_in {
                                div class="auth-status" {
                                    span style="color: var(--pico-ins-color);" { "Logged in" }
                                    form action="/api/logout" method="post" class="auth-form" {
                                        button type="submit" class="secondary outline" { "Logout" }
                                    }
                                }
                            } @else {
                                form action="/api/login" method="post" class="auth-form" {
                                    input type="password" name="password" placeholder="Password" required;
                                    button type="submit" { "Login" }
                                }
                            }
                        }
                    }

                    // Navigation
                    nav class="nav-buttons" {
                        a href="/" role="button" class=(if current_page == "match_day" { "primary" } else { "secondary outline" }) {
                            "Team Generator"
                        }
                        a href="/roster" role="button" class=(if current_page == "roster" { "primary" } else { "secondary outline" }) {
                            "Roster"
                        }
                        a href="/record" role="button" class=(if current_page == "record" { "primary" } else { "secondary outline" }) {
                            "Record Result"
                        }
                        a href="/history" role="button" class=(if current_page == "history" { "primary" } else { "secondary outline" }) {
                            "History"
                        }
                    }

                    hr;

                    // Page content
                    (content)
                }
            }
        }
    }
}

/// Render player tags as badges
pub fn render_tags(tags: &str) -> Markup {
    let tag_list: Vec<_> = tags.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    html! {
        @for tag in tag_list {
            span class="tag" { (tag) }
        }
    }
}

/// Format Elo delta with color
pub fn render_elo_delta(delta: f32) -> Markup {
    let sign = if delta >= 0.0 { "+" } else { "" };
    let class = if delta >= 0.0 { "elo-positive" } else { "elo-negative" };
    html! {
        span class=(class) { (sign) (format!("{:.0}", delta)) }
    }
}
