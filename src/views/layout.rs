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
                    .nav-buttons a { flex: 1; text-align: center; min-width: 70px; display: flex; align-items: center; justify-content: center; }
                    .team-grid { display: grid; grid-template-columns: 1fr; gap: 1rem; }
                    @media (min-width: 768px) { .team-grid { grid-template-columns: 1fr 1fr; } }
                    .player-list { list-style: none; padding: 0; }
                    .table-container { overflow-x: auto; }
                    .player-list li { padding: 0.5rem; border-bottom: 1px solid var(--pico-muted-border-color); }
                    .tag { display: inline-block; padding: 0.1rem 0.4rem; border-radius: 4px; font-size: 0.75rem; background: var(--pico-primary-background); color: var(--pico-primary-inverse); margin-left: 0.25rem; }
                    .elo-positive { color: var(--pico-ins-color); }
                    .elo-negative { color: var(--pico-del-color); }
                    .cost-breakdown { font-size: 0.875rem; color: var(--pico-muted-color); }
                    .checkbox-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap: 0.5rem; }
                    .header-row { display: flex; flex-direction: column; align-items: flex-start; gap: 0.5rem; margin-bottom: 1rem; }
                    @media (min-width: 768px) { .header-row { flex-direction: row; justify-content: space-between; align-items: center; gap: 1rem; } }
                    .auth-form, .auth-status { margin-left: auto; }
                    .auth-form { display: flex; gap: 0.5rem; align-items: stretch; margin: 0; }
                    .auth-form input, .auth-form button { margin: 0; padding: 0.5rem 0.75rem; height: auto; }
                    .auth-form input { width: 150px; max-width: 40vw; }
                    .auth-status { display: flex; gap: 0.5rem; align-items: center; }
                    .success-message {
                        color: var(--pico-ins-color);
                        font-weight: bold;
                        animation: fadeOut 3s forwards;
                    }
                    @keyframes fadeOut {
                        0% { opacity: 1; }
                        70% { opacity: 1; }
                        100% { opacity: 0; }
                    }
                    .htmx-indicator { display: none; }
                    .htmx-request .htmx-indicator, .htmx-request.htmx-indicator { display: inline-block; }
                    .spinner {
                        display: inline-block;
                        width: 1em;
                        height: 1em;
                        border: 2px solid currentColor;
                        border-right-color: transparent;
                        border-radius: 50%;
                        animation: spin 0.75s linear infinite;
                        vertical-align: middle;
                        margin-left: 0.5rem;
                    }
                    @keyframes spin { to { transform: rotate(360deg); } }
                    .page-title { margin: 0; }
                    .login-hint { margin-top: 0.5rem; font-size: 0.875rem; }
                    .logged-in-text { color: var(--pico-ins-color); }
                    .site-footer { margin-top: 2rem; padding-top: 1rem; border-top: 1px solid var(--pico-muted-border-color); text-align: center; }
                    .chart-container { position: relative; height: 400px; margin-bottom: 2rem; }
                    .participation-pct { font-size: 0.8em; }
                    .score-grid { align-items: center; }
                    .score-separator { text-align: center; font-size: 2rem; }
                    "#
                }
            }
            body {
                main class="container" {
                    // Header with title and auth
                    div class="header-row" {
                        h1 class="page-title" { "Sunday Football Manager" }

                        @if auth.enabled {
                            @if auth.logged_in {
                                div class="auth-status" {
                                    span class="logged-in-text" { "Logged in" }
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
                            "Teams"
                        }
                        a href="/roster" role="button" class=(if current_page == "roster" { "primary" } else { "secondary outline" }) {
                            "Roster"
                        }
                        a href="/record" role="button" class=(if current_page == "record" { "primary" } else { "secondary outline" }) {
                            "Record"
                        }
                        a href="/history" role="button" class=(if current_page == "history" { "primary" } else { "secondary outline" }) {
                            "History"
                        }
                    }

                    hr;

                    // Page content
                    (content)

                    // Footer
                    footer class="site-footer" {
                        small class="secondary" {
                            "Sunday Football Manager v" (env!("CARGO_PKG_VERSION"))
                            " · "
                            a href="https://github.com/ferologics/sunday-football-manager/blob/master/CHANGELOG.md" target="_blank" {
                                "Changelog"
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Render player tags as badges
pub fn render_tags(tags: &str) -> Markup {
    let tag_list: Vec<_> = tags
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    html! {
        @for tag in tag_list {
            span class="tag" { (tag) }
        }
    }
}

/// Format Elo delta with color
pub fn render_elo_delta(delta: f32) -> Markup {
    let sign = if delta >= 0.0 { "+" } else { "" };
    let class = if delta >= 0.0 {
        "elo-positive"
    } else {
        "elo-negative"
    };
    html! {
        span class=(class) { (sign) (format!("{:.0}", delta)) }
    }
}

/// Format participation percentage (only shown if < 100%)
pub fn render_participation(participation: f32) -> Markup {
    if participation < 1.0 {
        html! {
            span class="secondary participation-pct" {
                " (" (format!("{:.0}%", participation * 100.0)) ")"
            }
        }
    } else {
        html! {}
    }
}
