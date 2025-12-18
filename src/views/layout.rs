use maud::{html, Markup, DOCTYPE};

/// Base layout wrapper for all pages
pub fn base(title: &str, current_page: &str, content: Markup) -> Markup {
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
                    "#
                }
            }
            body {
                main class="container" {
                    h1 { "Sunday Football Manager" }

                    // Navigation
                    nav class="nav-buttons" {
                        a href="/" role="button" class=(if current_page == "match_day" { "primary" } else { "secondary outline" }) {
                            "Match Day"
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
