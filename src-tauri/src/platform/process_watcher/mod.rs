#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessInfo {
  pub pid: u64,
  pub path: String,
  pub arguments: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProcessEvent {
  Started(ProcessInfo),
  Exited { pid: u64 },
  Resync,
}

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use windows::*;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::*;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::*;

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
mod unsupported;
#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
pub use unsupported::*;
