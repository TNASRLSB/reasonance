use crate::agent_event::AgentEvent;
use crate::error::ReasonanceError;
use crate::transport::request::ImageAttachment;
use log::{debug, info};

/// Build an Anthropic Messages API request body with multimodal content.
pub fn build_anthropic_body(
    prompt: &str,
    model: &str,
    images: &[ImageAttachment],
    max_tokens: u64,
) -> serde_json::Value {
    let mut content_parts = vec![serde_json::json!({
        "type": "text",
        "text": prompt
    })];
    for img in images {
        content_parts.push(serde_json::json!({
            "type": "image",
            "source": {
                "type": "base64",
                "media_type": img.mime_type,
                "data": img.data
            }
        }));
    }
    serde_json::json!({
        "model": model,
        "max_tokens": max_tokens,
        "messages": [{
            "role": "user",
            "content": content_parts
        }]
    })
}

/// Build a Google Generative AI request body with inline image data.
pub fn build_google_body(
    prompt: &str,
    _model: &str,
    images: &[ImageAttachment],
) -> serde_json::Value {
    let mut parts = vec![serde_json::json!({ "text": prompt })];
    for img in images {
        parts.push(serde_json::json!({
            "inlineData": {
                "mimeType": img.mime_type,
                "data": img.data
            }
        }));
    }
    serde_json::json!({
        "contents": [{
            "parts": parts
        }]
    })
}

/// Build an OpenAI Chat Completions request body with image_url content parts.
pub fn build_openai_body(
    prompt: &str,
    model: &str,
    images: &[ImageAttachment],
) -> serde_json::Value {
    let mut content_parts = vec![serde_json::json!({
        "type": "text",
        "text": prompt
    })];
    for img in images {
        content_parts.push(serde_json::json!({
            "type": "image_url",
            "image_url": {
                "url": format!("data:{};base64,{}", img.mime_type, img.data)
            }
        }));
    }
    serde_json::json!({
        "model": model,
        "messages": [{
            "role": "user",
            "content": content_parts
        }]
    })
}

/// Send images to an LLM provider via direct REST API call.
///
/// Reads the API key from the environment variable named by `api_key_env`,
/// builds the appropriate multimodal request body for the provider, and
/// returns parsed response events (text content + optional usage).
pub async fn send_image_via_api(
    provider: &str,
    model: &str,
    prompt: &str,
    images: &[ImageAttachment],
    api_key_env: &str,
) -> Result<Vec<AgentEvent>, ReasonanceError> {
    let api_key = std::env::var(api_key_env).map_err(|_| {
        ReasonanceError::config(format!(
            "API key not configured. Set {} to send images with {}",
            api_key_env, provider
        ))
    })?;

    let client = reqwest::Client::new();
    let provider_lower = provider.to_lowercase();

    let (url, request_body, request_builder) = match provider_lower.as_str() {
        "claude" | "anthropic" => {
            let m = if model.is_empty() {
                "claude-sonnet-4-6"
            } else {
                model
            };
            let body = build_anthropic_body(prompt, m, images, 8192);
            let url = "https://api.anthropic.com/v1/messages".to_string();
            let rb = client
                .post(&url)
                .header("x-api-key", &api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json");
            (url, body, rb)
        }
        "gemini" | "google" => {
            let m = if model.is_empty() {
                "gemini-2.5-flash"
            } else {
                model
            };
            let url = format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
                m
            );
            let body = build_google_body(prompt, m, images);
            let rb = client
                .post(&url)
                .header("x-goog-api-key", &api_key)
                .header("content-type", "application/json");
            (url, body, rb)
        }
        _ => {
            let m = if model.is_empty() { "gpt-4o" } else { model };
            let body = build_openai_body(prompt, m, images);
            let url = "https://api.openai.com/v1/chat/completions".to_string();
            let rb = client
                .post(&url)
                .header("Authorization", format!("Bearer {}", api_key))
                .header("content-type", "application/json");
            (url, body, rb)
        }
    };

    info!(
        "direct_api: sending {} image(s) to {} via {}",
        images.len(),
        provider,
        url
    );
    debug!(
        "direct_api: request body keys: {:?}",
        request_body
            .as_object()
            .map(|o| o.keys().collect::<Vec<_>>())
    );

    let response = request_builder
        .json(&request_body)
        .send()
        .await
        .map_err(|e| {
            ReasonanceError::transport(provider, format!("HTTP request failed: {}", e), true)
        })?;

    let status = response.status();
    let data: serde_json::Value = response.json().await.map_err(|e| {
        ReasonanceError::transport(
            provider,
            format!("Failed to parse response JSON: {}", e),
            false,
        )
    })?;

    if !status.is_success() {
        let error_msg = data["error"]["message"]
            .as_str()
            .unwrap_or("Unknown API error");
        return Err(ReasonanceError::transport(
            provider,
            format!("API returned {}: {}", status, error_msg),
            status.is_server_error(),
        ));
    }

    let mut events = Vec::new();

    // Extract text content based on provider
    let text = match provider_lower.as_str() {
        "claude" | "anthropic" => data["content"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string(),
        "gemini" | "google" => data["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string(),
        _ => data["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string(),
    };

    if !text.is_empty() {
        events.push(AgentEvent::text(&text, provider));
    }

    // Extract token usage based on provider
    let (input_tokens, output_tokens) = match provider_lower.as_str() {
        "claude" | "anthropic" => (
            data["usage"]["input_tokens"].as_u64(),
            data["usage"]["output_tokens"].as_u64(),
        ),
        "gemini" | "google" => (
            data["usageMetadata"]["promptTokenCount"].as_u64(),
            data["usageMetadata"]["candidatesTokenCount"].as_u64(),
        ),
        _ => (
            data["usage"]["prompt_tokens"].as_u64(),
            data["usage"]["completion_tokens"].as_u64(),
        ),
    };

    if let (Some(input), Some(output)) = (input_tokens, output_tokens) {
        events.push(AgentEvent::usage(input, output, provider));
    }

    info!(
        "direct_api: received {} event(s) from {}",
        events.len(),
        provider
    );

    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_anthropic_body_includes_image_block() {
        let images = vec![ImageAttachment {
            data: "iVBOR".to_string(),
            mime_type: "image/png".to_string(),
            name: "test.png".to_string(),
        }];
        let body = build_anthropic_body("Describe this", "claude-sonnet-4-6", &images, 8192);
        let content = &body["messages"][0]["content"];
        assert_eq!(content[0]["type"], "text");
        assert_eq!(content[0]["text"], "Describe this");
        assert_eq!(content[1]["type"], "image");
        assert_eq!(content[1]["source"]["type"], "base64");
        assert_eq!(content[1]["source"]["media_type"], "image/png");
        assert_eq!(content[1]["source"]["data"], "iVBOR");
    }

    #[test]
    fn build_google_body_includes_inline_data() {
        let images = vec![ImageAttachment {
            data: "abc123".to_string(),
            mime_type: "image/jpeg".to_string(),
            name: "photo.jpg".to_string(),
        }];
        let body = build_google_body("What is this?", "gemini-2.5-pro", &images);
        let parts = &body["contents"][0]["parts"];
        assert_eq!(parts[0]["text"], "What is this?");
        assert_eq!(parts[1]["inlineData"]["mimeType"], "image/jpeg");
        assert_eq!(parts[1]["inlineData"]["data"], "abc123");
    }

    #[test]
    fn build_openai_body_includes_image_url() {
        let images = vec![ImageAttachment {
            data: "xyz".to_string(),
            mime_type: "image/png".to_string(),
            name: "s.png".to_string(),
        }];
        let body = build_openai_body("Look", "gpt-4o", &images);
        let content = &body["messages"][0]["content"];
        assert_eq!(content[0]["type"], "text");
        assert_eq!(content[1]["type"], "image_url");
        assert!(content[1]["image_url"]["url"]
            .as_str()
            .unwrap()
            .starts_with("data:image/png;base64,"));
    }
}
