use rocket::http::{ContentType, Status};
use rocket::serde::json::Json;
use rocket::{post, State};
use serde::{Deserialize, Serialize};

use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct StylesRequest {
    pub html: String,
    pub theme: String,
}

#[derive(Debug, Serialize)]
pub struct StylesResponse {
    pub css: String,
    pub theme: String,
    pub source: &'static str, // "cache" | "llm" | "fallback"
}

/// Validates a theme slug. Allowed: lowercase ASCII letters and digits, 1..=10 chars.
/// This bounds prompt-injection surface, makes the slug filesystem/Redis-safe, and
/// limits cache cardinality.
pub fn validate_theme(s: &str) -> Result<String, &'static str> {
    let s = s.trim();
    if s.is_empty() {
        return Err("theme is empty");
    }
    if s.len() > 10 {
        return Err("theme must be at most 10 characters");
    }
    if !s
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
    {
        return Err("theme must be a single lowercase alphanumeric token");
    }
    Ok(s.to_string())
}

#[post("/styles", data = "<req>")]
pub async fn post_styles(
    req: Json<StylesRequest>,
    state: &State<AppState>,
) -> Result<(ContentType, String), (Status, String)> {
    let req = req.into_inner();

    let theme = validate_theme(&req.theme).map_err(|e| (Status::BadRequest, e.to_string()))?;

    if let Some(cached) = state.cache.get(&theme).await {
        return respond(StylesResponse {
            css: cached,
            theme,
            source: "cache",
        });
    }

    let (css, source) = match crate::llm::generate_css(&req.html, &theme).await {
        Ok(css) => (css, "llm"),
        Err(e) => {
            eprintln!("[styles] LLM generation failed: {e:#}");
            (fallback_css(&theme), "fallback")
        }
    };

    if source == "llm" {
        if let Err(e) = state.cache.put(&theme, &css).await {
            eprintln!("[styles] failed to write cache: {e:#}");
        }
    }

    respond(StylesResponse { css, theme, source })
}

fn respond(body: StylesResponse) -> Result<(ContentType, String), (Status, String)> {
    let s =
        serde_json::to_string(&body).map_err(|e| (Status::InternalServerError, e.to_string()))?;
    Ok((ContentType::JSON, s))
}

pub fn fallback_css(theme: &str) -> String {
    let (bg, fg, accent, card) = match theme {
        t if t.contains("dark") => ("#0f1115", "#e8eaf0", "#7aa2ff", "#1a1d24"),
        t if t.contains("warm") || t.contains("sunset") => {
            ("#fff7ed", "#3b1d0c", "#ea580c", "#fff1e0")
        }
        t if t.contains("forest") || t.contains("nature") => {
            ("#f3f7f1", "#1d2a1d", "#2f7a3a", "#ffffff")
        }
        _ => ("#fafafa", "#111827", "#2563eb", "#ffffff"),
    };

    format!(
        r#"
:root {{
  --bg: {bg};
  --fg: {fg};
  --accent: {accent};
  --card: {card};
}}
* {{ box-sizing: border-box; }}
html, body {{ margin: 0; padding: 0; background: var(--bg); color: var(--fg); font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; line-height: 1.6; }}
.site-header {{ min-height: 100vh; display: flex; flex-direction: column; align-items: center; justify-content: center; text-align: center; padding: 2rem; background: linear-gradient(135deg, var(--card), var(--bg)); }}
.site-header .name {{ font-size: clamp(2.5rem, 6vw, 4.5rem); margin: 0; letter-spacing: -0.02em; }}
.site-header .title {{ font-size: 1.25rem; opacity: 0.75; margin: 0.5rem 0 2rem; }}
.download-resume {{ background: var(--accent); color: white; border: none; padding: 0.9rem 1.8rem; border-radius: 999px; font-size: 1rem; cursor: pointer; transition: transform 0.15s ease; }}
.download-resume:hover {{ transform: translateY(-2px); }}
section {{ max-width: 960px; margin: 0 auto; padding: 4rem 1.5rem; }}
section h2 {{ font-size: 2rem; margin: 0 0 1.5rem; border-bottom: 2px solid var(--accent); padding-bottom: 0.5rem; display: inline-block; }}
.about {{ display: flex; gap: 2rem; align-items: flex-start; flex-wrap: wrap; }}
.about .profile-pic {{ width: 160px; height: 160px; border-radius: 50%; object-fit: cover; border: 4px solid var(--accent); }}
.about .description {{ flex: 1 1 300px; }}
.experience-list {{ list-style: none; padding: 0; margin: 0; border-left: 3px solid var(--accent); }}
.experience-item {{ position: relative; padding: 0.5rem 0 1.5rem 1.5rem; }}
.experience-item::before {{ content: ""; position: absolute; left: -9px; top: 1rem; width: 15px; height: 15px; border-radius: 50%; background: var(--accent); }}
.experience-item .role {{ font-weight: 600; font-size: 1.1rem; }}
.experience-item .company {{ opacity: 0.8; }}
.experience-item .dates {{ font-size: 0.9rem; opacity: 0.6; }}
.experience-item .achievements {{ margin-top: 0.5rem; padding-left: 1.2rem; }}
.projects-grid {{ display: grid; grid-template-columns: repeat(auto-fill, minmax(260px, 1fr)); gap: 1.5rem; }}
.project-card {{ background: var(--card); border-radius: 12px; overflow: hidden; box-shadow: 0 4px 12px rgba(0,0,0,0.06); transition: transform 0.2s ease; text-decoration: none; color: inherit; display: flex; flex-direction: column; }}
.project-card:hover {{ transform: translateY(-4px); }}
.project-card img {{ width: 100%; aspect-ratio: 16/9; object-fit: cover; background: var(--bg); }}
.project-card .project-body {{ padding: 1rem; }}
.project-card .project-title {{ margin: 0 0 0.5rem; font-size: 1.15rem; }}
.project-card .project-description {{ margin: 0; opacity: 0.85; font-size: 0.95rem; }}
.site-footer {{ text-align: center; padding: 2rem; opacity: 0.8; }}
.site-footer a {{ display: inline-block; margin: 0 0.75rem; color: var(--fg); text-decoration: none; font-weight: 600; }}
.site-footer a:hover {{ color: var(--accent); }}
.theme-control {{ position: fixed; top: 1rem; right: 1rem; display: flex; gap: 0.5rem; background: var(--card); padding: 0.5rem; border-radius: 999px; box-shadow: 0 2px 8px rgba(0,0,0,0.1); z-index: 100; }}
.theme-control input {{ border: none; background: transparent; color: var(--fg); padding: 0.25rem 0.5rem; outline: none; min-width: 140px; }}
.theme-control button {{ background: var(--accent); color: white; border: none; padding: 0.4rem 0.9rem; border-radius: 999px; cursor: pointer; }}
"#
    )
}

#[cfg(test)]
mod tests {
    use super::validate_theme;

    #[test]
    fn accepts_lowercase_alnum() {
        assert_eq!(validate_theme("dark").unwrap(), "dark");
        assert_eq!(validate_theme("warm2").unwrap(), "warm2");
        assert_eq!(validate_theme("a").unwrap(), "a");
        assert_eq!(validate_theme("0123456789").unwrap(), "0123456789");
    }

    #[test]
    fn rejects_uppercase_and_symbols() {
        assert!(validate_theme("Dark").is_err());
        assert!(validate_theme("dark-mode").is_err());
        assert!(validate_theme("dark mode").is_err());
        assert!(validate_theme("dark!").is_err());
        assert!(validate_theme("../../etc").is_err());
    }

    #[test]
    fn rejects_too_long_or_empty() {
        assert!(validate_theme("").is_err());
        assert!(validate_theme("   ").is_err());
        assert!(validate_theme("12345678901").is_err());
    }
}
