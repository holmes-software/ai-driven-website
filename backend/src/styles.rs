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
    pub source: &'static str, // "cache" | "llm"
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

    let css = match crate::llm::generate_css(&req.html, &theme).await {
        Ok(css) => css,
        Err(e) => {
            eprintln!("[styles] LLM generation failed: {e:#}");
            return Err((Status::ServiceUnavailable, format!("LLM unavailable: {e}")));
        }
    };

    if let Err(e) = state.cache.put(&theme, &css).await {
        eprintln!("[styles] failed to write cache: {e:#}");
    }

    respond(StylesResponse {
        css,
        theme,
        source: "llm",
    })
}

fn respond(body: StylesResponse) -> Result<(ContentType, String), (Status, String)> {
    let s =
        serde_json::to_string(&body).map_err(|e| (Status::InternalServerError, e.to_string()))?;
    Ok((ContentType::JSON, s))
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
