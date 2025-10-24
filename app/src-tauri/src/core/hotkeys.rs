use anyhow::anyhow;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tracing::warn;

use crate::core::app_state::AppState;
use crate::core::events;

pub const DEFAULT_SHORTCUT: &str = "Ctrl+Space";

pub fn register(app: &AppHandle) -> tauri::Result<()> {
    if let Some(state) = app.try_state::<AppState>() {
        state.complete_session(app);
    }

    if let Err(error) = app
        .global_shortcut()
        .unregister(DEFAULT_SHORTCUT)
        .map_err(|err| anyhow!(err.to_string()))
    {
        warn!("failed to unregister existing hotkey: {error:?}");
    }

    app.global_shortcut()
        .on_shortcut(DEFAULT_SHORTCUT, move |app, _shortcut, event| {
            let state = app.state::<AppState>();
            let mode = state.hotkey_mode();
            match mode.as_str() {
                "toggle" => {
                    if matches!(event.state, ShortcutState::Pressed) {
                        if state.is_listening() {
                            state.mark_processing(app);
                            state.complete_session(app);
                        } else {
                            state.start_session(app);
                        }
                    }
                }
                _ => match event.state {
                    ShortcutState::Pressed => {
                        state.start_session(app);
                    }
                    ShortcutState::Released => {
                        if state.is_listening() {
                            state.mark_processing(app);
                        }
                        state.complete_session(app);
                    }
                },
            }
        })
        .map_err(|error| tauri::Error::from(anyhow!(error.to_string())))?;

    events::emit_hud_state(app, "idle");
    app.emit("hotkey-registered", DEFAULT_SHORTCUT)?;
    Ok(())
}

pub fn unregister(app: &AppHandle) -> tauri::Result<()> {
    if let Err(error) = app
        .global_shortcut()
        .unregister(DEFAULT_SHORTCUT)
        .map_err(|err| anyhow!(err.to_string()))
    {
        warn!("failed to unregister hotkey: {error:?}");
    }
    app.emit("hotkey-unregistered", DEFAULT_SHORTCUT)?;
    Ok(())
}
