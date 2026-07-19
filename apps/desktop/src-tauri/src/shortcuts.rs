//! Global keyboard shortcuts.

use std::str::FromStr;

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::commands::AppState;
use crate::overlays::{self, Overlay};
use crate::session::SessionController;

#[derive(Debug, Clone, Copy)]
enum Action {
    Stop,
    TogglePause,
    ToggleCaptions,
    AskAssistant,
}

/// Register the shortcuts from settings, replacing any already registered.
pub fn apply(app: &AppHandle) {
    let manager = app.global_shortcut();
    manager.unregister_all().ok();

    let shortcuts = app.state::<AppState>().settings.get().shortcuts;
    register(app, shortcuts.stop_session, Action::Stop);
    register(app, shortcuts.toggle_pause, Action::TogglePause);
    register(app, shortcuts.toggle_captions, Action::ToggleCaptions);
    register(app, shortcuts.ask_assistant, Action::AskAssistant);
}

fn register(app: &AppHandle, accelerator: Option<String>, action: Action) {
    let Some(accelerator) = accelerator else {
        return;
    };
    let Ok(shortcut) = Shortcut::from_str(&accelerator) else {
        tracing::warn!(%accelerator, "could not parse a keyboard shortcut; skipping");
        return;
    };

    let handle = app.clone();
    let result = app
        .global_shortcut()
        .on_shortcut(shortcut, move |_app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                run(&handle, action);
            }
        });

    if let Err(error) = result {
        tracing::warn!(%accelerator, %error, "could not register a keyboard shortcut");
    } else {
        tracing::info!(%accelerator, ?action, "registered a keyboard shortcut");
    }
}

fn run(app: &AppHandle, action: Action) {
    let session = app.state::<SessionController>();
    tracing::info!(?action, "keyboard shortcut fired");

    match action {
        Action::Stop => {
            if session.status().is_some() {
                session.stop(app).ok();
            }
        }
        Action::TogglePause => {
            if let Some(status) = session.status() {
                let pause = status.state == audis_common::SessionState::Listening;
                session.set_paused(app, pause).ok();
            }
        }
        Action::ToggleCaptions => {
            if session.status().is_some() {
                overlays::toggle(app, Overlay::Captions);
            }
        }
        Action::AskAssistant => {
            if session.status().is_none() {
                return;
            }
            if !app.state::<AppState>().settings.get().assistant.enabled {
                return;
            }

            overlays::place(app, Overlay::Assistant);
            app.emit(audis_common::events::ASSISTANT_ASK, ()).ok();
        }
    }
}
