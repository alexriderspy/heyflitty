//! Flitty: a screen-aware AI buddy that points at things.
//!
//! Hold the hotkey and ask; Flitty transcribes the question, looks at the
//! screens, answers out loud and flies its cursor to what it is talking about.

mod audio;
mod capture;
mod commands;
mod cursor;
mod elements;
mod openai;
mod overlay;
mod pipeline;
mod platform;
mod pointing;
mod prompt;
mod screens;
mod settings;
mod tray;

use tauri::Manager;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

fn push_to_talk_shortcut() -> Shortcut {
    // Ctrl+Alt+Space; Ctrl+Option+Space on a Mac.
    Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::Space)
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
                        ShortcutState::Pressed => pipeline::on_press(app),
                        ShortcutState::Released => pipeline::on_release(app),
                    }
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![commands::get_settings, commands::save_settings, commands::test_key])
        .setup(move |app| {
            // Lives in the menu bar: no Dock icon, never takes activation from other apps.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let handle = app.handle().clone();
            handle.manage(settings::SettingsStore::load(&handle)?);
            handle.manage(pipeline::Assistant::new());
            let screen_list = screens::all(&handle);
            println!("[flitty] screens: {screen_list:?}");
            overlay::create_all(&handle, &screen_list)?;
            overlay::start_topmost_guard(handle.clone(), screen_list.clone());
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
