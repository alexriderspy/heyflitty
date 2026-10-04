//! Commands the settings panel calls. The key goes in, but only "is a key saved" comes out.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

use crate::openai;
use crate::settings::{self, Settings, SettingsStore};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsView {
    settings: Settings,
    has_key: bool,
}

#[tauri::command]
pub fn get_settings(store: State<SettingsStore>) -> SettingsView {
    SettingsView { settings: store.get(), has_key: settings::read_key().is_some() }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveRequest {
    settings: Settings,
    /// None keeps the saved key; an empty string removes it.
    api_key: Option<String>,
}

#[tauri::command]
pub fn save_settings(app: AppHandle, store: State<SettingsStore>, request: SaveRequest) -> Result<SettingsView, String> {
    if let Some(key) = request.api_key {
        settings::write_key(key.trim())?;
    }
    store.save(request.settings)?;
    let _ = app.emit("settings-changed", ());
    Ok(get_settings(store))
}

#[tauri::command]
pub async fn test_key() -> Result<(), String> {
    let key = settings::read_key().ok_or("no OpenAI API key saved")?;
    openai::check_key(&reqwest::Client::new(), &key).await
}
