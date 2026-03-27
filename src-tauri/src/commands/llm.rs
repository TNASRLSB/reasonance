use crate::error::ReasonanceError;
use log::{debug, error, info};
use serde::Serialize;

#[derive(Serialize)]
struct LlmResult {
    content: Option<String>,
    error: Option<String>,
}

fn ok_result(content: String) -> String {
    serde_json::to_string(&LlmResult {
        content: Some(content),
        error: None,
    })
    .unwrap_or_else(|e| format!(r#"{{"content":null,"error":"serialize: {}"}}"#, e))
}

fn err_result(error: String) -> String {
    serde_json::to_string(&LlmResult {
        content: None,
        error: Some(error.clone()),
    })
    .unwrap_or_else(|_| format!(r#"{{"content":null,"error":"{}"}}"#, error))
}

#[tauri::command]
pub async fn call_llm_api(
    provider: String,
    model: String,
    prompt: String,
    api_key_env: String,
    endpoint: String,
) -> Result<String, ReasonanceError> {
    info!("cmd::call_llm_api(provider={}, model={})", provider, model);
    // Read the API key from environment — it never leaves the backend
    let api_key = if api_key_env.is_empty() {
        String::new()
    } else {
        std::env::var(&api_key_env).unwrap_or_default()
    };

    let client = reqwest::Client::new();

    let result = match provider.as_str() {
        "anthropic" => call_anthropic(&client, &model, &prompt, &api_key).await,
        "google" => call_google(&client, &model, &prompt, &api_key).await,
        _ => call_openai(&client, &model, &prompt, &api_key, &endpoint).await,
    };

    match result {
        Ok(ref content) => {
            debug!(
                "cmd::call_llm_api success for provider={}, response_len={}",
                provider,
                content.len()
            );
        }
        Err(ref e) => {
            error!("cmd::call_llm_api error for provider={}: {}", provider, e);
        }
    }
    match result {
        Ok(content) => Ok(ok_result(content)),
        Err(e) => Ok(err_result(e)),
    }
}

async fn call_anthropic(
    client: &reqwest::Client,
    model: &str,
    prompt: &str,
    api_key: &str,
) -> Result<String, String> {
    let body = serde_json::json!({
        "model": if model.is_empty() { "claude-sonnet-4-6" } else { model },
        "max_tokens": 4096,
        "messages": [{"role": "user", "content": prompt}]
    });

    let res = client
        .post("https://api.anthropic.com/v1/messages")
        .header("Content-Type", "application/json")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        let status = res.status().as_u16();
        let text = res.text().await.unwrap_or_default();
        return Err(format!("{}: {}", status, text));
    }

    let data: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
    Ok(data["content"][0]["text"]
        .as_str()
        .unwrap_or("")
        .to_string())
}

async fn call_openai(
    client: &reqwest::Client,
    model: &str,
    prompt: &str,
    api_key: &str,
    endpoint: &str,
) -> Result<String, String> {
    let base = if endpoint.is_empty() {
        "https://api.openai.com/v1"
    } else {
        endpoint.trim_end_matches('/')
    };
    let url = format!("{}/chat/completions", base);

    let body = serde_json::json!({
        "model": if model.is_empty() { "gpt-4o" } else { model },
        "messages": [{"role": "user", "content": prompt}]
    });

    let mut req = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&body);

    if !api_key.is_empty() {
        req = req.header("Authorization", format!("Bearer {}", api_key));
    }

    let res = req.send().await.map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        let status = res.status().as_u16();
        let text = res.text().await.unwrap_or_default();
        return Err(format!("{}: {}", status, text));
    }

    let data: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
    Ok(data["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string())
}

async fn call_google(
    client: &reqwest::Client,
    model: &str,
    prompt: &str,
    api_key: &str,
) -> Result<String, String> {
    let m = if model.is_empty() {
        "gemini-pro"
    } else {
        model
    };
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
        m
    );

    let body = serde_json::json!({
        "contents": [{"parts": [{"text": prompt}]}]
    });

    let res = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("x-goog-api-key", api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        let status = res.status().as_u16();
        let text = res.text().await.unwrap_or_default();
        return Err(format!("{}: {}", status, text));
    }

    let data: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
    Ok(data["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .unwrap_or("")
        .to_string())
}
