//! The push-to-talk loop: record → transcribe → screenshot → model → speak → point.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::audio::Recorder;
use crate::capture::{self, ScreenCapture};
use crate::chat::{self, ChatRequest, Exchange};
use crate::overlay::{self, label_for, PointTarget, VoiceState};
use crate::pointing::{self, SentenceSplitter};
use crate::screens::{self, Screen};
use crate::settings::{self, KeyKind, SettingsStore};
use crate::transcribe;
use crate::{prompt, tray};

const HISTORY_LIMIT: usize = 10;
const MAX_REPLY_TOKENS: u32 = 1024;

pub struct Assistant {
    recorder: Recorder,
    history: Mutex<Vec<Exchange>>,
    http: reqwest::Client,
    /// Bumped on every press; a running turn stops as soon as it is stale.
    turn: AtomicU64,
}

impl Assistant {
    pub fn new() -> Self {
        Self { recorder: Recorder::new(), history: Mutex::new(Vec::new()), http: reqwest::Client::new(), turn: AtomicU64::new(0) }
    }
}

#[derive(Clone, Serialize)]
struct SpeakPayload {
    text: String,
}

#[derive(Clone, Serialize)]
struct NoticePayload {
    text: String,
}

/// Debug builds can skip the microphone and transcription with a fixed question,
/// so the rest of the loop can be tested in a VM without audio.
fn test_transcript() -> Option<String> {
    if cfg!(debug_assertions) {
        std::env::var("FLITTY_TEST_TRANSCRIPT").ok().filter(|text| !text.is_empty())
    } else {
        None
    }
}

pub fn on_press(app: &AppHandle) {
    let assistant = app.state::<Assistant>();
    assistant.turn.fetch_add(1, Ordering::SeqCst);
    let _ = app.emit("stop-speech", ());
    overlay::set_voice_state(app, VoiceState::Listening);
    if test_transcript().is_none() {
        if let Err(error) = assistant.recorder.start() {
            notify(app, &format!("I can't hear you: {error}"));
        }
    }
}

pub fn on_release(app: &AppHandle) {
    let app = app.clone();
    let turn = app.state::<Assistant>().turn.load(Ordering::SeqCst);
    tauri::async_runtime::spawn(async move {
        if let Err(error) = run_turn(&app, turn).await {
            if is_current(&app, turn) {
                if crate::errors::points_to_settings(&error) {
                    tray::open_settings(&app);
                }
                notify(&app, &error);
            }
        }
        if is_current(&app, turn) {
            let _ = app.emit("response-complete", ());
        }
    });
}

fn is_current(app: &AppHandle, turn: u64) -> bool {
    app.state::<Assistant>().turn.load(Ordering::SeqCst) == turn
}

async fn run_turn(app: &AppHandle, turn: u64) -> Result<(), String> {
    let assistant = app.state::<Assistant>();
    let settings = app.state::<SettingsStore>().get();
    let started = Instant::now();

    let question = match test_transcript() {
        Some(text) => text,
        None => {
            let recording = match assistant.recorder.stop() {
                Ok(recording) => recording,
                Err(error) if error == "too short" => {
                    overlay::set_voice_state(app, VoiceState::Idle);
                    return Ok(());
                }
                Err(error) => return Err(format!("recording failed: {error}")),
            };
            overlay::set_voice_state(app, VoiceState::Processing);
            let key = settings::transcription_key(&settings);
            if key.is_none() && needs_key(&settings.transcription.base_url) {
                tray::open_settings(app);
                return Err(format!("add a {} key under Speech to text so I can understand you", crate::errors::provider_name(&settings.transcription.base_url)));
            }
            let text = transcribe::transcribe(&assistant.http, &settings.transcription, key.as_deref(), recording.wav).await?;
            println!("[flitty] heard ({:.1}s audio) in {:?}: {text}", recording.seconds, started.elapsed());
            text
        }
    };
    if question.is_empty() || !is_current(app, turn) {
        overlay::set_voice_state(app, VoiceState::Idle);
        return Ok(());
    }
    overlay::set_voice_state(app, VoiceState::Processing);

    let chat_key = settings::read_key(KeyKind::Chat);
    if chat_key.is_none() && needs_key(&settings.chat.base_url) {
        tray::open_settings(app);
        return Err(format!("add your {} key under AI model first", crate::errors::provider_name(&settings.chat.base_url)));
    }

    let screen_list = screens::all(app);
    let cursor_screen = screens::under_cursor(app, &screen_list).or_else(|| screen_list.first().cloned()).ok_or("no screens found")?;
    let captures = tauri::async_runtime::spawn_blocking(capture::capture_all)
        .await
        .map_err(|error| error.to_string())??;
    let matched: Vec<(&ScreenCapture, Screen)> = captures.iter().filter_map(|capture| match_screen(capture, &screen_list).map(|screen| (capture, screen))).collect();
    let labeled = matched
        .iter()
        .enumerate()
        .map(|(position, (capture, screen))| {
            let focus = if screen.index == cursor_screen.index { " (cursor screen)" } else { "" };
            (*capture, format!("screen {} of {}{focus}, image is {}x{} pixels", position + 1, matched.len(), capture.image_width, capture.image_height))
        })
        .collect::<Vec<_>>();

    let history: Vec<Exchange> = assistant
        .history
        .lock()
        .expect("history lock")
        .iter()
        .map(|exchange| Exchange { user: exchange.user.clone(), assistant: exchange.assistant.clone() })
        .collect();

    let mut splitter = SentenceSplitter::new();
    let mut started_speaking = false;
    let voice_muted = settings.voice.muted;
    let speak_target = label_for(cursor_screen.index);
    let mut speak = |sentence: String| {
        if !is_current(app, turn) {
            return;
        }
        if !started_speaking {
            started_speaking = true;
            println!("[flitty] first sentence after {:?}", started.elapsed());
            overlay::set_voice_state(app, VoiceState::Responding);
        }
        if !voice_muted {
            let _ = app.emit_to(speak_target.as_str(), "speak", SpeakPayload { text: sentence });
        }
    };

    let reply = chat::stream_reply(
        &assistant.http,
        &settings.chat,
        chat_key.as_deref(),
        ChatRequest { system_prompt: prompt::SYSTEM_PROMPT, history: &history, question: &question, screens: labeled, max_tokens: MAX_REPLY_TOKENS },
        |delta| {
            for sentence in splitter.push(delta) {
                speak(sentence);
            }
        },
    )
    .await?;
    if let Some(rest) = splitter.finish() {
        speak(rest);
    }
    if !is_current(app, turn) {
        return Ok(());
    }
    println!("[flitty] reply complete after {:?}: {reply}", started.elapsed());

    let (spoken, tag) = pointing::parse(&reply);
    if let Some(tag) = tag {
        let target = tag
            .screen_number
            .and_then(|number| matched.get(number.wrapping_sub(1)))
            .or_else(|| matched.iter().find(|(_, screen)| screen.index == cursor_screen.index));
        if let Some((capture, screen)) = target {
            let x = (tag.x.clamp(0.0, capture.image_width as f64) / capture.image_width as f64) * screen.css_width();
            let y = (tag.y.clamp(0.0, capture.image_height as f64) / capture.image_height as f64) * screen.css_height();
            overlay::point_at(app, screen, PointTarget { x, y, label: tag.label });
        }
    }
    if !started_speaking {
        overlay::set_voice_state(app, VoiceState::Idle);
    }

    let mut history = assistant.history.lock().expect("history lock");
    history.push(Exchange { user: question, assistant: spoken });
    let overflow = history.len().saturating_sub(HISTORY_LIMIT);
    history.drain(..overflow);
    Ok(())
}

/// xcap and Tauri list monitors independently; match them by origin.
fn match_screen(capture: &ScreenCapture, screen_list: &[Screen]) -> Option<Screen> {
    screen_list
        .iter()
        .find(|screen| {
            let physical_match = screen.x == capture.monitor_x && screen.y == capture.monitor_y;
            let logical_match = (screen.x as f64 / screen.scale).round() as i32 == capture.monitor_x
                && (screen.y as f64 / screen.scale).round() as i32 == capture.monitor_y;
            physical_match || logical_match
        })
        .cloned()
}

/// Hosted providers always need a key; self-hosted endpoints (Ollama, LM Studio) usually don't.
pub fn needs_key(base_url: &str) -> bool {
    ["anthropic.com", "openai.com", "openrouter.ai", "googleapis.com", "groq.com", "x.ai", "mistral.ai"]
        .iter()
        .any(|host| base_url.contains(host))
}

fn notify(app: &AppHandle, text: &str) {
    eprintln!("[flitty] {text}");
    overlay::set_voice_state(app, VoiceState::Idle);
    let cursor_screen = screens::under_cursor(app, &screens::all(app));
    let target = cursor_screen.map(|screen| label_for(screen.index)).unwrap_or_else(|| label_for(0));
    let _ = app.emit_to(target.as_str(), "notice", NoticePayload { text: text.to_string() });
}
