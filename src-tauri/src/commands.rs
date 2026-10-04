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
