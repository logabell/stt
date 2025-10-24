#[cfg(debug_assertions)]
use crate::output::logs;
#[cfg(all(target_os = "windows", feature = "windows-accessibility"))]
use crate::output::win_access;
use serde::{Deserialize, Serialize};
use tracing::warn;

#[cfg(target_os = "windows")]
mod windows_clipboard;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OutputAction {
    Paste,
    Copy,
}

#[derive(Default)]
pub struct OutputInjector;

impl OutputInjector {
    pub fn new() -> Self {
        Self
    }

    pub fn inject(&self, text: &str, action: OutputAction) {
        match action {
            OutputAction::Paste => {
                #[cfg(all(target_os = "windows", feature = "windows-accessibility"))]
                {
                    if win_access::focused_control_is_secure().unwrap_or(false) {
                        warn!("Skipping paste into secure field");
                        return;
                    }
                }
                #[cfg(target_os = "windows")]
                {
                    if let Err(error) = windows_clipboard::paste_preserving_clipboard(text) {
                        warn!("Paste failed: {error}");
                    }
                }

                #[cfg(not(target_os = "windows"))]
                {
                    warn!("Simulated paste: {}", text);
                }

                #[cfg(debug_assertions)]
                logs::push_log(format!("Paste -> {}", text));
            }
            OutputAction::Copy => {
                warn!("Copy injector not yet implemented, text: {}", text);
            }
        }
    }
}
