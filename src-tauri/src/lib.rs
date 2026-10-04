//! Flitty: a screen-aware AI buddy that points at things.
//!
//! This is the foundation slice: per-display overlays, a cursor buddy that
//! follows the mouse, a tray icon, and a push-to-talk hotkey that captures the
//! screens and flies to a placeholder target. Voice and the model come next.

mod capture;
mod cursor;
mod overlay;
mod platform;
mod screens;
mod tray;

use std::thread;

use tauri::AppHandle;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use overlay::{PointTarget, VoiceState};

fn push_to_talk_shortcut() -> Shortcut {
    // Ctrl+Alt+Space; Ctrl+Option+Space on a Mac.
    Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::Space)
}

fn on_push_to_talk_released(app: AppHandle) {
    thread::spawn(move || {
        let screen_list = screens::all(&app);
        let cursor_screen = screens::under_cursor(&app, &screen_list);
        overlay::set_voice_state(&app, VoiceState::Processing);

        if let Err(error) = capture::capture_all() {
            eprintln!("[flitty] capture failed: {error}");
        }

        overlay::set_voice_state(&app, VoiceState::Idle);
        // Placeholder until the model picks the target: the top-right toolbar area.
        if let Some(screen) = cursor_screen {
            let target = PointTarget { x: screen.css_width() * 0.8, y: 60.0, label: "right here".into() };
            overlay::point_at(&app, &screen, target);
        }
    });
}

pub fn run() {
    let push_to_talk = push_to_talk_shortcut();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| tray::open_settings(app)))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, shortcut, event| {
                    if shortcut != &push_to_talk {
                        return;
                    }
                    match event.state() {
                        ShortcutState::Pressed => overlay::set_voice_state(app, VoiceState::Listening),
                        ShortcutState::Released => on_push_to_talk_released(app.clone()),
                    }
                })
                .build(),
        )
        .setup(move |app| {
            // Lives in the menu bar: no Dock icon, never takes activation from other apps.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let handle = app.handle().clone();
            let screen_list = screens::all(&handle);
            println!("[flitty] screens: {screen_list:?}");
            overlay::create_all(&handle, &screen_list)?;
            cursor::start_stream(handle.clone(), screen_list);
            tray::install(&handle)?;
            handle.global_shortcut().register(push_to_talk)?;
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building Flitty")
        .run(|_app, event| {
            // Closing the settings panel must not quit a tray app.
            if let tauri::RunEvent::ExitRequested { api, code: None, .. } = event {
                api.prevent_exit();
            }
        });
}
