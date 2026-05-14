use anyhow::{anyhow, Context, Result};

fn build_messages_prompt(html: &str, theme: &str) -> String {
    format!(
        "Generate a complete, modern, responsive CSS stylesheet for the HTML below, \
         applying the theme: \"{theme}\".\n\n\
         Style every section (header, about, experience, projects, footer). \
         Use the existing class names and element structure exactly as written. \
         The header should feel hero-like with the name prominent. \
         Project cards should be visually distinct and arranged in a responsive grid. \
         Experience entries should read as a clear timeline. \
         Be tasteful: good typography, spacing, contrast, and a cohesive palette.\n\n\
         HTML:\n```html\n{html}\n```"
    )
}

const SYSTEM_PROMPT: &str = "You are a senior front-end designer. \
    You output ONLY raw CSS. \
    No markdown fences, no commentary, no explanations, no surrounding prose.";

const MODEL_NAME: &str = "grok-4-1-fast-reasoning";

fn strip_code_fences(s: &str) -> &str {
    let s = s.trim();
    if let Some(rest) = s.strip_prefix("```css") {
        return rest.trim().trim_end_matches("```").trim();
    }
    if let Some(rest) = s.strip_prefix("```") {
        return rest.trim().trim_end_matches("```").trim();
    }
    s
}

fn http_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .context("building HTTP client")
}

/// Generate a CSS stylesheet via Microsoft Foundry's chat-completions endpoint.
/// The model is selected on the Foundry side; this client is model-agnostic.
pub async fn generate_css(html: &str, theme: &str) -> Result<String> {
    let url =
        std::env::var("AZURE_FOUNDRY_URL").map_err(|_| anyhow!("AZURE_FOUNDRY_URL not set"))?;
    let api_key = std::env::var("AZURE_FOUNDRY_API_KEY")
        .map_err(|_| anyhow!("AZURE_FOUNDRY_API_KEY not set"))?;
    let auth = format!("Bearer {}", api_key);

    let body = serde_json::json!({
        "messages": [
            { "role": "system", "content": SYSTEM_PROMPT },
            { "role": "user", "content": build_messages_prompt(html, theme) }
        ],
        "max_completion_tokens": 8000,
        "temperature": 1,
        "top_p": 1,
        "model": MODEL_NAME,
    });

    let resp = http_client()?
        .post(&url)
        .header("authorization", &auth)
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .context("sending request to Azure Foundry")?;

    let status = resp.status();
    let text = resp.text().await.context("reading Azure Foundry body")?;
    if !status.is_success() {
        return Err(anyhow!("Azure Foundry returned {status}: {text}"));
    }

    let v: serde_json::Value =
        serde_json::from_str(&text).context("parsing Azure Foundry response as JSON")?;
    let content = v
        .get("choices")
        .and_then(|c| c.as_array())
        .and_then(|arr| arr.first())
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|t| t.as_str())
        .ok_or_else(|| anyhow!("unexpected Azure Foundry response shape: {text}"))?;

    Ok(strip_code_fences(content).to_string())
}

#[cfg(test)]
mod tests {
    use super::strip_code_fences;

    #[test]
    fn strips_css_fence() {
        assert_eq!(strip_code_fences("```css\nbody{}\n```"), "body{}");
    }

    #[test]
    fn strips_plain_fence() {
        assert_eq!(strip_code_fences("```\nbody{}\n```"), "body{}");
    }

    #[test]
    fn leaves_unfenced_alone() {
        assert_eq!(strip_code_fences("  body{}  "), "body{}");
    }
}
