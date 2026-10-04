//! Streaming vision chat against Anthropic's Messages API or any
//! OpenAI-compatible /chat/completions endpoint.

use base64::Engine;
use futures_util::StreamExt;
use serde_json::{json, Value};

use crate::capture::ScreenCapture;
use crate::settings::{ChatApi, ChatSettings};

pub struct Exchange {
    pub user: String,
    pub assistant: String,
}

pub struct ChatRequest<'a> {
    pub system_prompt: &'a str,
    pub history: &'a [Exchange],
    pub question: &'a str,
    /// Each screenshot with the label the model uses to refer to it.
    pub screens: Vec<(&'a ScreenCapture, String)>,
    pub max_tokens: u32,
}

/// Streams the reply, calling `on_text` for every text delta, and returns the full text.
pub async fn stream_reply(
    client: &reqwest::Client,
    settings: &ChatSettings,
    api_key: Option<&str>,
    request: ChatRequest<'_>,
    mut on_text: impl FnMut(&str),
) -> Result<String, String> {
    let base = settings.base_url.trim_end_matches('/');
    let (url, body) = match settings.api {
        ChatApi::Anthropic => (format!("{base}/v1/messages"), anthropic_body(settings, &request)),
        ChatApi::OpenAiCompatible => (format!("{base}/chat/completions"), openai_body(settings, &request)),
    };
    let mut http = client.post(&url).json(&body);
    if let Some(key) = api_key {
        http = match settings.api {
            ChatApi::Anthropic => http.header("x-api-key", key).header("anthropic-version", "2023-06-01"),
            ChatApi::OpenAiCompatible => http.bearer_auth(key),
        };
    }
    let response = http.send().await.map_err(|error| format!("model request failed: {error}"))?;
    let status = response.status();
    if !status.is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err(format!("model error ({status}): {}", text.chars().take(300).collect::<String>()));
    }

    let mut full_text = String::new();
    let mut pending = String::new();
    let mut bytes = response.bytes_stream();
    while let Some(chunk) = bytes.next().await {
        pending.push_str(&String::from_utf8_lossy(&chunk.map_err(|error| error.to_string())?));
        // Server-sent events are separated by blank lines; keep any partial event for the next chunk.
        while let Some(boundary) = pending.find("\n\n") {
            let event: String = pending.drain(..boundary + 2).collect();
            for line in event.lines() {
                let Some(data) = line.strip_prefix("data:").map(str::trim) else { continue };
                if data == "[DONE]" {
                    continue;
                }
                if let Some(delta) = text_delta(settings.api, data) {
                    full_text.push_str(&delta);
                    on_text(&delta);
                }
            }
        }
    }
    Ok(full_text)
}

fn text_delta(api: ChatApi, data: &str) -> Option<String> {
    let event: Value = serde_json::from_str(data).ok()?;
    let text = match api {
        ChatApi::Anthropic => (event["type"] == "content_block_delta").then(|| event["delta"]["text"].as_str()).flatten(),
        ChatApi::OpenAiCompatible => event["choices"][0]["delta"]["content"].as_str(),
    };
    text.filter(|text| !text.is_empty()).map(str::to_string)
}

fn encode(capture: &ScreenCapture) -> String {
    base64::engine::general_purpose::STANDARD.encode(&capture.jpeg)
}

fn anthropic_body(settings: &ChatSettings, request: &ChatRequest) -> Value {
    let mut messages = Vec::new();
    for exchange in request.history {
        messages.push(json!({ "role": "user", "content": exchange.user }));
        messages.push(json!({ "role": "assistant", "content": exchange.assistant }));
    }
    let mut content = Vec::new();
    for (capture, label) in &request.screens {
        content.push(json!({ "type": "text", "text": label }));
        content.push(json!({ "type": "image", "source": { "type": "base64", "media_type": "image/jpeg", "data": encode(capture) } }));
    }
    content.push(json!({ "type": "text", "text": request.question }));
    messages.push(json!({ "role": "user", "content": content }));
    json!({
        "model": settings.model,
        "max_tokens": request.max_tokens,
        "system": request.system_prompt,
        "stream": true,
        "messages": messages,
    })
}

fn openai_body(settings: &ChatSettings, request: &ChatRequest) -> Value {
    let mut messages = vec![json!({ "role": "system", "content": request.system_prompt })];
    for exchange in request.history {
        messages.push(json!({ "role": "user", "content": exchange.user }));
        messages.push(json!({ "role": "assistant", "content": exchange.assistant }));
    }
    let mut content = Vec::new();
    for (capture, label) in &request.screens {
        content.push(json!({ "type": "text", "text": label }));
        content.push(json!({ "type": "image_url", "image_url": { "url": format!("data:image/jpeg;base64,{}", encode(capture)) } }));
    }
    content.push(json!({ "type": "text", "text": request.question }));
    messages.push(json!({ "role": "user", "content": content }));
    json!({
        "model": settings.model,
        "max_tokens": request.max_tokens,
        "stream": true,
        "messages": messages,
    })
}
