use std::path::PathBuf;

#[cfg(target_os = "windows")]
pub(crate) fn config_root() -> PathBuf {
  dirs::data_dir().unwrap_or_default().join("acord")
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn config_root() -> PathBuf {
  dirs::config_dir().unwrap_or_default().join("acord")
}

#[cfg(target_os = "windows")]
pub(crate) fn user_content_root() -> PathBuf {
  dirs::home_dir().unwrap_or_default().join("acord")
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn user_content_root() -> PathBuf {
  config_root()
}
