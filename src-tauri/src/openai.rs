//! The OpenAI endpoints Flitty uses: transcription and streaming vision chat.

use base64::Engine;
use futures_util::StreamExt;
use reqwest::multipart::{Form, Part};
use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::capture::ScreenCapture;

/// Debug builds can point at the mock server in scripts/mock-ai for end-to-end tests.
pub fn base_url() -> String {
    if cfg!(debug_assertions) {
        if let Ok(url) = std::env::var("FLITTY_TEST_OPENAI_BASE_URL") {
            return url.trim_end_matches('/').to_string();
        }
    }
    "https://api.openai.com/v1".to_string()
}

/// Short phrases get misheard as other languages without a hint.
const TRANSCRIPTION_LANGUAGE: &str = "en";

pub struct Exchange {
    pub user: String,
    pub assistant: String,
}

pub struct ChatRequest<'a> {
    pub model: &'a str,
    pub system_prompt: &'a str,
    pub history: &'a [Exchange],
    pub question: &'a str,
    /// Each screenshot with the label the model uses to refer to it.
    pub screens: Vec<(&'a ScreenCapture, String)>,
    pub max_tokens: u32,
}

#[derive(Deserialize)]
struct TranscriptionResponse {
    text: String,
}

pub async fn transcribe(client: &reqwest::Client, api_key: &str, model: &str, wav: Vec<u8>) -> Result<String, String> {
    let audio = Part::bytes(wav).file_name("speech.wav").mime_str("audio/wav").map_err(|error| error.to_string())?;
    let form = Form::new().text("model", model.to_string()).text("language", TRANSCRIPTION_LANGUAGE).part("file", audio);
    let response = client
        .post(format!("{}/audio/transcriptions", base_url()))
        .bearer_auth(api_key)
        .multipart(form)
        .send()
        .await
        .map_err(|error| network_error(&error))?;
    let status = response.status();
    if !status.is_success() {
        return Err(http_error("speech-to-text", status, &response.text().await.unwrap_or_default()));
    }
    let parsed: TranscriptionResponse = response.json().await.map_err(|error| error.to_string())?;
    Ok(parsed.text.trim().to_string())
}

/// Streams the reply, calling `on_text` for every text delta, and returns the full text.
pub async fn stream_reply(client: &reqwest::Client, api_key: &str, request: ChatRequest<'_>, mut on_text: impl FnMut(&str)) -> Result<String, String> {
    let response = client
        .post(format!("{}/chat/completions", base_url()))
        .bearer_auth(api_key)
        .json(&chat_body(&request))
        .send()
        .await
        .map_err(|error| network_error(&error))?;
    let status = response.status();
    if !status.is_success() {
        return Err(http_error("model", status, &response.text().await.unwrap_or_default()));
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
                let parsed: Value = serde_json::from_str(data).map_err(|error| format!("unexpected stream data: {error}"))?;
                if let Some(delta) = parsed["choices"][0]["delta"]["content"].as_str().filter(|text| !text.is_empty()) {
                    full_text.push_str(delta);
                    on_text(delta);
                }
            }
        }
    }
    Ok(full_text)
}

/// Checks the key with a cheap authenticated request.
pub async fn check_key(client: &reqwest::Client, api_key: &str) -> Result<(), String> {
    let response = client.get(format!("{}/models", base_url())).bearer_auth(api_key).send().await.map_err(|error| network_error(&error))?;
    let status = response.status();
    if status.is_success() {
        Ok(())
    } else {
        Err(http_error("key check", status, &response.text().await.unwrap_or_default()))
    }
}

fn chat_body(request: &ChatRequest) -> Value {
    let mut messages = vec![json!({ "role": "system", "content": request.system_prompt })];
    for exchange in request.history {
        messages.push(json!({ "role": "user", "content": exchange.user }));
        messages.push(json!({ "role": "assistant", "content": exchange.assistant }));
    }
    let mut content = Vec::new();
    for (capture, label) in &request.screens {
        let image = base64::engine::general_purpose::STANDARD.encode(&capture.jpeg);
        content.push(json!({ "type": "text", "text": label }));
        content.push(json!({ "type": "image_url", "image_url": { "url": format!("data:image/jpeg;base64,{image}") } }));
    }
    content.push(json!({ "type": "text", "text": request.question }));
    messages.push(json!({ "role": "user", "content": content }));
    json!({ "model": request.model, "max_tokens": request.max_tokens, "stream": true, "messages": messages })
}

/// Phrase that tells the caller opening Settings will help.
pub const CHECK_SETTINGS: &str = "check it in Settings";

fn http_error(what: &str, status: StatusCode, body: &str) -> String {
    match status.as_u16() {
        401 | 403 => format!("OpenAI rejected your API key, {CHECK_SETTINGS}"),
        404 => format!("OpenAI doesn't know that model, {CHECK_SETTINGS}"),
        429 => "OpenAI says you're out of credits or sending too fast; try again in a moment".into(),
        500..=599 => "OpenAI is having trouble right now; try again in a moment".into(),
        _ => {
            eprintln!("[flitty] {what} error {status}: {}", body.chars().take(300).collect::<String>());
            format!("the {what} request failed ({status})")
        }
    }
}

fn network_error(error: &reqwest::Error) -> String {
    eprintln!("[flitty] network error: {error}");
    "I can't reach OpenAI; check your internet connection".into()
}
