#[cfg(all(target_os = "windows", feature = "windows-accessibility"))]
mod secure_field {
    use anyhow::{Context, Result};
    use windows::Win32::UI::Accessibility::{self, IUIAutomationElement, UiaGetFocusedElement};

    pub fn focused_control_is_secure() -> Result<bool> {
        unsafe {
            let mut element: Option<IUIAutomationElement> = None;
            UiaGetFocusedElement(&mut element)
                .ok()
                .context("UIAutomation not available")?;
            let element = element.context("no focused element")?;
            let mut control_type = 0i32;
            element.get_CurrentControlType(&mut control_type).ok()?;
            Ok(control_type == Accessibility::UIA_PasswordControlTypeId)
        }
    }
}

#[cfg(all(target_os = "windows", feature = "windows-accessibility"))]
pub use secure_field::focused_control_is_secure;

#[cfg(not(all(target_os = "windows", feature = "windows-accessibility")))]
pub fn focused_control_is_secure() -> anyhow::Result<bool> {
    Ok(false)
}
