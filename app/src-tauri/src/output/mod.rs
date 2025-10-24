mod injector;
#[cfg(debug_assertions)]
pub mod logs;
pub mod tray;
#[cfg(all(target_os = "windows", feature = "windows-accessibility"))]
pub mod win_access;

pub use injector::{OutputAction, OutputInjector};
