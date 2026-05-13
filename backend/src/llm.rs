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

/// Try Anthropic on Azure first, then Azure OpenAI. Returns the first success.
pub async fn generate_css(html: &str, theme: &str) -> Result<String> {
    let mut errors: Vec<String> = Vec::new();

    match generate_with_anthropic(html, theme).await {
        Ok(css) => return Ok(css),
        Err(e) => errors.push(format!("anthropic: {e:#}")),
    }

    match generate_with_azure_openai(html, theme).await {
        Ok(css) => return Ok(css),
        Err(e) => errors.push(format!("azure-openai: {e:#}")),
    }

    Err(anyhow!("all LLM providers failed: {}", errors.join(" | ")))
}

// ---------------------------------------------------------------------------
// Anthropic on Azure
// ---------------------------------------------------------------------------

async fn generate_with_anthropic(html: &str, theme: &str) -> Result<String> {
    let url =
        std::env::var("AZURE_ANTHROPIC_URL").map_err(|_| anyhow!("AZURE_ANTHROPIC_URL not set"))?;
    let api_key = std::env::var("AZURE_ANTHROPIC_API_KEY")
        .map_err(|_| anyhow!("AZURE_ANTHROPIC_API_KEY not set"))?;
    let model =
        std::env::var("AZURE_ANTHROPIC_MODEL").unwrap_or_else(|_| "claude-sonnet-4-7".to_string());

    let body = serde_json::json!({
        "model": model,
        "max_tokens": 8000,
        "system": SYSTEM_PROMPT,
        "messages": [{ "role": "user", "content": build_messages_prompt(html, theme) }],
        "anthropic_version": "vertex-2023-10-16",
    });

    let resp = http_client()?
        .post(&url)
        .header("api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .context("sending request to Azure Anthropic")?;

    let status = resp.status();
    let text = resp
        .text()
        .await
        .context("reading Anthropic response body")?;
    if !status.is_success() {
        return Err(anyhow!("Anthropic returned {status}: {text}"));
    }

    let v: serde_json::Value =
        serde_json::from_str(&text).context("parsing Anthropic response as JSON")?;
    let content = v
        .get("content")
        .and_then(|c| c.as_array())
        .and_then(|arr| arr.iter().find_map(|item| item.get("text")))
        .and_then(|t| t.as_str())
        .ok_or_else(|| anyhow!("unexpected Anthropic response shape: {text}"))?;

    Ok(strip_code_fences(content).to_string())
}

// ---------------------------------------------------------------------------
// Azure OpenAI
// ---------------------------------------------------------------------------

async fn generate_with_azure_openai(html: &str, theme: &str) -> Result<String> {
    let url = std::env::var("AZURE_OPENAI_URL").map_err(|_| anyhow!("AZURE_OPENAI_URL not set"))?;
    let api_key = std::env::var("AZURE_OPENAI_API_KEY")
        .map_err(|_| anyhow!("AZURE_OPENAI_API_KEY not set"))?;

    let body = serde_json::json!({
        "messages": [
            { "role": "system", "content": SYSTEM_PROMPT },
            { "role": "user", "content": build_messages_prompt(html, theme) }
        ],
        "max_tokens": 8000,
        "temperature": 0.7,
    });

    let resp = http_client()?
        .post(&url)
        .header("api-key", &api_key)
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .context("sending request to Azure OpenAI")?;

    let status = resp.status();
    let text = resp.text().await.context("reading Azure OpenAI body")?;
    if !status.is_success() {
        return Err(anyhow!("Azure OpenAI returned {status}: {text}"));
    }

    let v: serde_json::Value =
        serde_json::from_str(&text).context("parsing Azure OpenAI response as JSON")?;
    let content = v
        .get("choices")
        .and_then(|c| c.as_array())
        .and_then(|arr| arr.first())
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|t| t.as_str())
        .ok_or_else(|| anyhow!("unexpected Azure OpenAI response shape: {text}"))?;

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
