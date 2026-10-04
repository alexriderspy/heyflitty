//! The push-to-talk loop: record → transcribe → screenshot → model → speak → point.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::audio::Recorder;
use crate::capture::{self, ScreenCapture};
use crate::openai::{self, ChatRequest, Exchange};
use crate::overlay::{self, label_for, PointTarget, VoiceState};
use crate::elements::{self, UiElement};
use crate::pointing::{self, PointTarget as Target, SentenceSplitter};
use crate::screens::{self, Screen};
use crate::settings::{self, SettingsStore};
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
    // Chromium-based apps build their accessibility tree only once a UI Automation client
    // asks; asking now means the full tree is ready by the time the hotkey is released.
    std::thread::spawn(|| {
        let _ = elements::focused_window_elements();
    });
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
                if error.contains("Settings") {
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
    let Some(api_key) = settings::read_key() else {
        tray::open_settings(app);
        return Err("add your OpenAI API key in Settings first".into());
    };

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
            let text = openai::transcribe(&assistant.http, &api_key, &settings.transcription_model, recording.wav).await?;
            println!("[flitty] heard ({:.1}s audio) in {:?}: {text}", recording.seconds, started.elapsed());
            text
        }
    };
    if question.is_empty() || !is_current(app, turn) {
        overlay::set_voice_state(app, VoiceState::Idle);
        return Ok(());
    }
    if let Some(screen) = screens::under_cursor(app, &screens::all(app)) {
        let _ = app.emit_to(label_for(screen.index).as_str(), "heard", SpeakPayload { text: question.clone() });
    }
    overlay::set_voice_state(app, VoiceState::Processing);

    let screen_list = screens::all(app);
    let cursor_screen = screens::under_cursor(app, &screen_list).ok_or("I couldn't tell which screen the mouse is on")?;
    // Read the controls first: the screenshot shows the same moment.
    let ui_elements = tauri::async_runtime::spawn_blocking(elements::focused_window_elements)
        .await
        .map_err(|error| error.to_string())?
        .unwrap_or_else(|error| {
            eprintln!("[flitty] couldn't read UI elements: {error}");
            Vec::new()
        });
    // Where the mouse is and what it is over, so "what's this?" can mean the thing under the pointer.
    let mouse = app.cursor_position().ok();
    let under_mouse = match mouse {
        Some(point) => tauri::async_runtime::spawn_blocking(move || elements::element_under(point.x as i32, point.y as i32)).await.ok().flatten(),
        None => None,
    };
    let mouse_point = mouse.map(|point| (point.x, point.y));
    let captures = tauri::async_runtime::spawn_blocking(move || capture::capture_all(mouse_point))
        .await
        .map_err(|error| error.to_string())??;
    let matched = captures
        .iter()
        .map(|capture| match_screen(capture, &screen_list).map(|screen| (capture, screen)).ok_or("a captured display didn't match any known screen"))
        .collect::<Result<Vec<(&ScreenCapture, Screen)>, _>>()?;
    let labeled = matched
        .iter()
        .enumerate()
        .map(|(position, (capture, screen))| {
            let focus = match mouse.filter(|_| screen.index == cursor_screen.index) {
                Some(point) => {
                    let x = (point.x - screen.x as f64) / screen.width as f64 * capture.image_width as f64;
                    let y = (point.y - screen.y as f64) / screen.height as f64 * capture.image_height as f64;
                    let over = under_mouse.as_ref().map(|element| format!(", over {element}")).unwrap_or_default();
                    format!(" (cursor screen, mouse pointer at {},{}{over})", x.round(), y.round())
                }
                None => String::new(),
            };
            (*capture, format!("screen {} of {}{focus}, image is {}x{} pixels", position + 1, matched.len(), capture.image_width, capture.image_height))
        })
        .collect::<Vec<_>>();
    if cfg!(debug_assertions) {
        for (_, label) in &labeled {
            println!("[flitty] {label}");
        }
    }
    let element_list = describe_elements(&ui_elements, &matched);
    println!("[flitty] {} UI elements listed", ui_elements.len());
    if cfg!(debug_assertions) {
        if let Some(list) = &element_list {
            println!("[flitty] {list}");
        }
    }

    let history: Vec<Exchange> = assistant
        .history
        .lock()
        .expect("history lock")
        .iter()
        .map(|exchange| Exchange { user: exchange.user.clone(), assistant: exchange.assistant.clone() })
        .collect();

    let mut splitter = SentenceSplitter::new();
    let mut started_speaking = false;
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
        let _ = app.emit_to(speak_target.as_str(), "speak", SpeakPayload { text: sentence });
    };

    let reply = openai::stream_reply(
        &assistant.http,
        &api_key,
        ChatRequest { model: &settings.chat_model, system_prompt: prompt::SYSTEM_PROMPT, history: &history, question: &question, screens: labeled, context: element_list, max_tokens: MAX_REPLY_TOKENS },
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
        match tag.target {
            Target::Element(id) => match ui_elements.iter().find(|element| element.id == id) {
                Some(element) => {
                    let (center_x, center_y) = element.center();
                    match screen_list.iter().find(|screen| screen.contains(center_x, center_y)) {
                        Some(screen) => {
                            let (x, y) = screen.to_local_css(center_x, center_y);
                            overlay::point_at(app, screen, PointTarget { x, y, label: tag.label });
                        }
                        None => eprintln!("[flitty] element #{id} is not on any screen"),
                    }
                }
                None => eprintln!("[flitty] model pointed at unknown element #{id}"),
            },
            Target::Pixels { x: image_x, y: image_y, screen_number } => {
                // No screen number means the cursor screen, per the prompt; an unknown number points nowhere.
                let target = match screen_number {
                    Some(number) => matched.get(number.wrapping_sub(1)),
                    None => matched.iter().find(|(_, screen)| screen.index == cursor_screen.index),
                };
                if let Some((capture, screen)) = target {
                    let x = (image_x.clamp(0.0, capture.image_width as f64) / capture.image_width as f64) * screen.css_width();
                    let y = (image_y.clamp(0.0, capture.image_height as f64) / capture.image_height as f64) * screen.css_height();
                    overlay::point_at(app, screen, PointTarget { x, y, label: tag.label });
                }
            }
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

/// One line per element, with its center in the pixels of the screenshot it appears in.
fn describe_elements(ui_elements: &[UiElement], matched: &[(&ScreenCapture, Screen)]) -> Option<String> {
    let lines: Vec<String> = ui_elements
        .iter()
        .filter_map(|element| {
            let (center_x, center_y) = element.center();
            let (position, (capture, screen)) = matched.iter().enumerate().find(|(_, (_, screen))| screen.contains(center_x, center_y))?;
            let image_x = (center_x - screen.x as f64) / screen.width as f64 * capture.image_width as f64;
            let image_y = (center_y - screen.y as f64) / screen.height as f64 * capture.image_height as f64;
            Some(format!("[#{}] {} \"{}\" at {},{} on screen {}", element.id, element.role, element.name, image_x.round(), image_y.round(), position + 1))
        })
        .collect();
    (!lines.is_empty()).then(|| format!("clickable elements in the focused window and the taskbar:\n{}", lines.join("\n")))
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


fn notify(app: &AppHandle, text: &str) {
    eprintln!("[flitty] {text}");
    overlay::set_voice_state(app, VoiceState::Idle);
    let payload = NoticePayload { text: text.to_string() };
    match screens::under_cursor(app, &screens::all(app)) {
        Some(screen) => {
            let _ = app.emit_to(label_for(screen.index).as_str(), "notice", payload);
        }
        // Mouse position unknown: show it on every screen rather than guess one.
        None => {
            let _ = app.emit("notice", payload);
        }
    }
}
