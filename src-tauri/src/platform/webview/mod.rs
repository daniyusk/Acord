#[cfg(target_os = "windows")]
#[path = "windows.rs"]
mod imp;

#[cfg(target_os = "linux")]
#[path = "linux.rs"]
mod imp;

#[cfg(target_os = "macos")]
#[path = "macos.rs"]
mod imp;

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
#[path = "unsupported.rs"]
mod imp;

pub(crate) use imp::{configure_after_creation, configure_before_creation};
