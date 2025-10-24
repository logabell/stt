#[cfg(all(target_os = "windows", feature = "windows-accessibility"))]
mod paste {
  use anyhow::{Context, Result};
  use windows::Win32::UI::Input::KeyboardAndMouse::{SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VK_CONTROL, VK_KEY_V};

  pub fn send_ctrl_v() -> Result<()> {
    unsafe {
      let mut inputs: [INPUT; 4] = [std::mem::zeroed(); 4];

      inputs[0].r#type = INPUT_KEYBOARD;
      inputs[0].Anonymous.ki = KEYBDINPUT {
        wVk: VK_CONTROL.0,
        ..Default::default()
      };

      inputs[1].r#type = INPUT_KEYBOARD;
      inputs[1].Anonymous.ki = KEYBDINPUT {
        wVk: VK_KEY_V.0,
        ..Default::default()
      };

      inputs[2].r#type = INPUT_KEYBOARD;
      inputs[2].Anonymous.ki = KEYBDINPUT {
        wVk: VK_KEY_V.0,
        dwFlags: KEYEVENTF_KEYUP,
        ..Default::default()
      };

      inputs[3].r#type = INPUT_KEYBOARD;
      inputs[3].Anonymous.ki = KEYBDINPUT {
        wVk: VK_CONTROL.0,
        dwFlags: KEYEVENTF_KEYUP,
        ..Default::default()
      };

      SendInput(&inputs, std::mem::size_of::<INPUT>() as i32).ok().context("SendInput failed")?
    }
    Ok(())
  }
}

#[cfg(all(target_os = "windows", feature = "windows-accessibility"))]
pub use paste::send_ctrl_v;

#[cfg(not(all(target_os = "windows", feature = "windows-accessibility")))]
pub fn send_ctrl_v() -> anyhow::Result<()> {
    Ok(())
}
