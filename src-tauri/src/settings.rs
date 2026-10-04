//! User settings. Preferences live in a JSON file in the app config directory;
//! the OpenAI API key lives in the OS credential store (Windows Credential
//! Manager, macOS Keychain) and never reaches the webviews.

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

const KEYRING_SERVICE: &str = "com.heyflitty.desktop";
const KEYRING_ACCOUNT: &str = "openai-api-key";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub chat_model: String,
    pub transcription_model: String,
    /// Name of a system voice; empty uses the best English voice installed.
    pub voice_name: String,
    pub muted: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            chat_model: "gpt-4.1-mini".into(),
            transcription_model: "gpt-4o-mini-transcribe".into(),
            voice_name: String::new(),
            muted: false,
        }
    }
}

pub struct SettingsStore {
    path: PathBuf,
    current: Mutex<Settings>,
}

impl SettingsStore {
    /// A missing file means first run; an unreadable one is an error, not a silent reset.
    pub fn load(app: &AppHandle) -> Result<Self, String> {
        let path = app.path().app_config_dir().map_err(|error| error.to_string())?.join("settings.json");
        let current = match fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(text.trim_start_matches('\u{feff}'))
                .map_err(|error| format!("{} is not valid settings JSON: {error}", path.display()))?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Settings::default(),
            Err(error) => return Err(format!("can't read {}: {error}", path.display())),
        };
        Ok(Self { path, current: Mutex::new(current) })
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

fn entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT).map_err(|error| error.to_string())
}

pub fn read_key() -> Option<String> {
    // Debug builds can use a throwaway key for tests without touching the saved one;
    // set but empty means "no key".
    if cfg!(debug_assertions) {
        if let Ok(test_key) = std::env::var("FLITTY_TEST_API_KEY") {
            return (!test_key.trim().is_empty()).then_some(test_key);
        }
    }
    entry().ok()?.get_password().ok().filter(|key| !key.is_empty())
}

pub fn write_key(key: &str) -> Result<(), String> {
    let entry = entry()?;
    if key.is_empty() {
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(error.to_string()),
        }
    } else {
        entry.set_password(key).map_err(|error| error.to_string())
    }
}
