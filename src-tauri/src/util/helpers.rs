use super::paths::*;
use base64::{engine::general_purpose, Engine as _};
use std::{path::*, process::Command, sync::OnceLock};

use crate::log;
use crate::util::input_validation::{validate_http_url, MAX_REMOTE_RESPONSE_BYTES};

static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

pub fn get_http_client() -> &'static reqwest::Client {
  HTTP_CLIENT.get_or_init(|| {
    reqwest::Client::builder()
      .redirect(reqwest::redirect::Policy::none())
      .build()
      .unwrap_or_else(|_| reqwest::Client::new())
  })
}

#[tauri::command]
pub async fn fetch_image(url: String) -> Option<String> {
  let url = validate_http_url(&url).ok()?;
  let client = get_http_client();
  let response = client
    .get(url)
    .header("User-Agent", "Acord")
    .send()
    .await
    .ok()?;

  if !response.status().is_success()
    || response
      .content_length()
      .is_some_and(|length| length > MAX_REMOTE_RESPONSE_BYTES)
  {
    return None;
  }

  // extract the content type
  let content_type = response
    .headers()
    .get("content-type")
    .and_then(|value| value.to_str().ok())
    .map(|s| s.to_owned())
    .unwrap_or_else(|| {
      eprintln!("Error: Unable to get content type");
      String::new()
    });

  if !content_type.to_ascii_lowercase().starts_with("image/") {
    return None;
  }

  let bytes = response.bytes().await.ok()?;
  if bytes.len() > MAX_REMOTE_RESPONSE_BYTES as usize {
    return None;
  }
  let base64 = general_purpose::STANDARD.encode(bytes);
  let image = format!("data:{content_type};base64,{base64}");

  Some(image)
}

#[tauri::command]
pub fn open_plugins() {
  let plugin_folder = get_plugin_dir();

  open_folder(plugin_folder).unwrap_or_default()
}

#[tauri::command]
pub fn open_themes() {
  let theme_folder = get_theme_dir();

  open_folder(theme_folder).unwrap_or_default()
}

#[tauri::command]
pub fn open_extensions() {
  let extension_folder = get_extensions_dir();

  open_folder(extension_folder).unwrap_or_default()
}

fn open_folder(path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
  open::that(path)?;
  Ok(())
}

#[tauri::command]
pub fn get_platform() -> &'static str {
  #[cfg(target_os = "windows")]
  return "windows";

  #[cfg(target_os = "macos")]
  return "macos";

  #[cfg(target_os = "linux")]
  "linux"
}

#[tauri::command]
pub fn restart_in_safemode(app: tauri::AppHandle) {
  let current_exe = match std::env::current_exe() {
    Ok(current_exe) => current_exe,
    Err(e) => {
      log!("Failed to resolve current executable for safemode restart: {e:?}");
      return;
    }
  };

  match Command::new(current_exe).arg("--safemode").spawn() {
    Ok(_) => app.exit(0),
    Err(e) => log!("Failed to restart Acord in safemode: {e:?}"),
  }
}

#[cfg(target_os = "windows")]
pub fn is_windows_7() -> bool {
  use windows::{
    Wdk::System::SystemServices::RtlGetVersion, Win32::System::SystemInformation::OSVERSIONINFOW,
  };

  let mut osvi = OSVERSIONINFOW {
    dwOSVersionInfoSize: std::mem::size_of::<OSVERSIONINFOW>() as u32,
    ..Default::default()
  };

  unsafe {
    let _ = RtlGetVersion(&mut osvi);
  }

  osvi.dwMajorVersion == 6 && osvi.dwMinorVersion == 1
}
