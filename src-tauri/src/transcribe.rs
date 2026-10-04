//! Speech-to-text through an OpenAI-compatible /audio/transcriptions endpoint
//! (OpenAI, Groq, a local whisper server, ...).

use reqwest::multipart::{Form, Part};
use serde::Deserialize;

use crate::settings::TranscriptionSettings;

#[derive(Deserialize)]
struct TranscriptionResponse {
    text: String,
}

pub async fn transcribe(
    client: &reqwest::Client,
    settings: &TranscriptionSettings,
    api_key: Option<&str>,
    wav: Vec<u8>,
) -> Result<String, String> {
    let url = format!("{}/audio/transcriptions", settings.base_url.trim_end_matches('/'));
    let audio = Part::bytes(wav).file_name("speech.wav").mime_str("audio/wav").map_err(|error| error.to_string())?;
    let mut form = Form::new().text("model", settings.model.clone()).part("file", audio);
    if !settings.language.trim().is_empty() {
        form = form.text("language", settings.language.trim().to_string());
    }
    let mut request = client.post(&url).multipart(form);
    if let Some(key) = api_key {
        request = request.bearer_auth(key);
    }
    let response = request.send().await.map_err(|error| crate::errors::network("speech-to-text", &settings.base_url, &error))?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(crate::errors::http("speech-to-text", &settings.base_url, status, &body));
    }
    let parsed: TranscriptionResponse = response.json().await.map_err(|error| error.to_string())?;
    Ok(parsed.text.trim().to_string())
}
