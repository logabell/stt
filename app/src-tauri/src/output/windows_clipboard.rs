#[cfg(target_os = "windows")]
mod platform {
    use super::windows_paste::send_ctrl_v;
    use anyhow::{Context, Result};
    use clipboard_win::{formats, get_clipboard, set_clipboard, Clipboard, ClipboardGuard};

    fn open_clipboard_try() -> Result<ClipboardGuard> {
        Clipboard::new_attempts(5)
            .map_err(|error| anyhow::anyhow!("clipboard open failed: {error}"))
    }

    fn snapshot_clipboard() -> Result<Option<String>> {
        let guard = open_clipboard_try()?;
        let current: Option<String> = get_clipboard(formats::Unicode)?.into();
        drop(guard);
        Ok(current)
    }

    fn restore_clipboard(value: Option<String>) -> Result<()> {
        let guard = open_clipboard_try()?;
        match value {
            Some(text) => set_clipboard(formats::Unicode, text)?,
            None => set_clipboard(formats::Unicode, "")?,
        };
        drop(guard);
        Ok(())
    }

    pub fn paste_preserving_clipboard(text: &str) -> Result<()> {
        // snapshot existing clipboard
        let snapshot = snapshot_clipboard().context("snapshot clipboard")?;

        // set clipboard to new text
        {
            let guard = open_clipboard_try()?;
            set_clipboard(formats::Unicode, text).context("set clipboard")?;
            drop(guard);
        }

        let paste_result = send_ctrl_v();

        // restore
        restore_clipboard(snapshot).context("restore clipboard")?;
        paste_result.context("send ctrl+v keystroke")
    }
}

#[cfg(target_os = "windows")]
pub use platform::paste_preserving_clipboard;

#[cfg(not(target_os = "windows"))]
pub fn paste_preserving_clipboard(_text: &str) -> anyhow::Result<()> {
    Ok(())
}
