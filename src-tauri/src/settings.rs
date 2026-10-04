//! User settings. Plain preferences live in a JSON file in the app config
//! directory; API keys live in the OS credential store (Windows Credential
//! Manager, macOS Keychain) and never reach the webviews.

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

const KEYRING_SERVICE: &str = "com.heyflitty.desktop";

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ChatApi {
    #[default]
    Anthropic,
    /// OpenAI, OpenRouter, Gemini's OpenAI endpoint, Ollama, LM Studio, ...
    OpenAiCompatible,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ChatSettings {
    pub api: ChatApi,
    pub base_url: String,
    pub model: String,
}

impl Default for ChatSettings {
    fn default() -> Self {
        Self { api: ChatApi::Anthropic, base_url: "https://api.anthropic.com".into(), model: "claude-sonnet-5-5".into() }
    }
}

/// Speech-to-text through any OpenAI-compatible /audio/transcriptions endpoint.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TranscriptionSettings {
    pub base_url: String,
    pub model: String,
    /// ISO-639-1 code like "en"; empty lets the service guess, which mishears short phrases.
    pub language: String,
}

impl Default for TranscriptionSettings {
    fn default() -> Self {
        Self { base_url: "https://api.openai.com/v1".into(), model: "gpt-4o-mini-transcribe".into(), language: "en".into() }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct VoiceSettings {
    /// Name of a system voice; empty picks the best available one.
    pub voice_name: String,
    pub muted: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub chat: ChatSettings,
    pub transcription: TranscriptionSettings,
    pub voice: VoiceSettings,
}

/// Which secret a key belongs to.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum KeyKind {
    Chat,
    Transcription,
}

impl KeyKind {
    fn account(self) -> &'static str {
        match self {
            KeyKind::Chat => "chat-api-key",
            KeyKind::Transcription => "transcription-api-key",
        }
    }
}

pub struct SettingsStore {
    path: PathBuf,
    current: Mutex<Settings>,
}

impl SettingsStore {
    pub fn load(app: &AppHandle) -> Self {
        let path = app
            .path()
            .app_config_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("settings.json");
        let current = fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str(text.trim_start_matches('\u{feff}')).ok())
            .unwrap_or_default();
        Self { path, current: Mutex::new(current) }
    }

    pub fn get(&self) -> Settings {
        self.current.lock().expect("settings lock").clone()
    }

    pub fn save(&self, settings: Settings) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let text = serde_json::to_string_pretty(&settings).map_err(|error| error.to_string())?;
        fs::write(&self.path, text).map_err(|error| error.to_string())?;
        *self.current.lock().expect("settings lock") = settings;
        Ok(())
    }
}

fn entry(kind: KeyKind) -> Result<keyring::Entry, String> {
    keyring::Entry::new(KEYRING_SERVICE, kind.account()).map_err(|error| error.to_string())
}

pub fn read_key(kind: KeyKind) -> Option<String> {
    entry(kind).ok()?.get_password().ok().filter(|key| !key.is_empty())
}

pub fn write_key(kind: KeyKind, key: &str) -> Result<(), String> {
    let entry = entry(kind)?;
    if key.is_empty() {
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(error.to_string()),
        }
    } else {
        entry.set_password(key).map_err(|error| error.to_string())
    }
}

/// The transcription key falls back to the chat key only when both point at the
/// same provider, so one OpenAI key covers both but an Anthropic key is never sent to OpenAI.
pub fn transcription_key(settings: &Settings) -> Option<String> {
    read_key(KeyKind::Transcription).or_else(|| {
        let same_host = host_of(&settings.chat.base_url) == host_of(&settings.transcription.base_url);
        if same_host { read_key(KeyKind::Chat) } else { None }
    })
}

pub fn host_of(url: &str) -> String {
    url.split("://").nth(1).unwrap_or(url).split(['/', ':']).next().unwrap_or("").to_lowercase()
}
