//! Commands the settings panel calls. Keys go in, but only "is a key saved" comes out.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

use crate::settings::{self, KeyKind, Settings, SettingsStore};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsView {
    settings: Settings,
    has_chat_key: bool,
    has_transcription_key: bool,
}

#[tauri::command]
pub fn get_settings(store: State<SettingsStore>) -> SettingsView {
    SettingsView {
        settings: store.get(),
        has_chat_key: settings::read_key(KeyKind::Chat).is_some(),
        has_transcription_key: settings::read_key(KeyKind::Transcription).is_some(),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveRequest {
    settings: Settings,
    /// None keeps the saved key; an empty string removes it.
    chat_key: Option<String>,
    transcription_key: Option<String>,
}

#[tauri::command]
pub fn save_settings(app: AppHandle, store: State<SettingsStore>, request: SaveRequest) -> Result<SettingsView, String> {
    if let Some(key) = request.chat_key {
        settings::write_key(KeyKind::Chat, key.trim())?;
    }
    if let Some(key) = request.transcription_key {
        settings::write_key(KeyKind::Transcription, key.trim())?;
    }
    store.save(request.settings)?;
    let _ = app.emit("settings-changed", ());
    Ok(get_settings(store))
}

#[derive(Serialize)]
pub struct KeyCheck {
    ok: bool,
    message: String,
}

#[derive(Serialize)]
pub struct KeyChecks {
    chat: KeyCheck,
    transcription: KeyCheck,
}

/// Checks the saved keys with a cheap authenticated request (listing models) to each provider.
#[tauri::command]
pub async fn test_keys(store: State<'_, SettingsStore>) -> Result<KeyChecks, String> {
    let current = store.get();
    let client = reqwest::Client::new();

    let chat_base = current.chat.base_url.trim_end_matches('/').to_string();
    let chat_key = settings::read_key(KeyKind::Chat);
    let chat_request = match current.chat.api {
        settings::ChatApi::Anthropic => {
            let request = client.get(format!("{chat_base}/v1/models")).header("anthropic-version", "2023-06-01");
            match &chat_key { Some(key) => request.header("x-api-key", key), None => request }
        }
        settings::ChatApi::OpenAiCompatible => {
            let request = client.get(format!("{chat_base}/models"));
            match &chat_key { Some(key) => request.bearer_auth(key), None => request }
        }
    };
    let chat = check("AI model", &current.chat.base_url, chat_key.is_some(), chat_request).await;

    let transcription_base = current.transcription.base_url.trim_end_matches('/').to_string();
    let transcription_key = settings::transcription_key(&current);
    let transcription_request = client.get(format!("{transcription_base}/models"));
    let transcription_request = match &transcription_key { Some(key) => transcription_request.bearer_auth(key), None => transcription_request };
    let transcription = check("speech-to-text", &current.transcription.base_url, transcription_key.is_some(), transcription_request).await;

    Ok(KeyChecks { chat, transcription })
}

async fn check(service: &str, base_url: &str, has_key: bool, request: reqwest::RequestBuilder) -> KeyCheck {
    if !has_key && crate::pipeline::needs_key(base_url) {
        return KeyCheck { ok: false, message: format!("no {} key saved", crate::errors::provider_name(base_url)) };
    }
    match request.send().await {
        Ok(response) if response.status().is_success() => KeyCheck { ok: true, message: format!("{} accepted the key", crate::errors::provider_name(base_url)) },
        Ok(response) => {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            KeyCheck { ok: false, message: crate::errors::http(service, base_url, status, &body) }
        }
        Err(error) => KeyCheck { ok: false, message: crate::errors::network(service, base_url, &error) },
    }
}
