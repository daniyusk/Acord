use crate::log;

use super::paths::get_webdata_dir;

const MIN_ZOOM_LEVEL: f64 = 0.25;
const MAX_ZOOM_LEVEL: f64 = 5.0;

fn sanitize_zoom_level(value: f64) -> f64 {
  if value.is_finite() {
    value.clamp(MIN_ZOOM_LEVEL, MAX_ZOOM_LEVEL)
  } else {
    1.0
  }
}

#[cfg(test)]
mod tests {
  use super::sanitize_zoom_level;

  #[test]
  fn clamps_invalid_zoom_values() {
    assert_eq!(sanitize_zoom_level(1.25), 1.25);
    assert_eq!(sanitize_zoom_level(0.1), 0.25);
    assert_eq!(sanitize_zoom_level(6.0), 5.0);
    assert_eq!(sanitize_zoom_level(f64::NAN), 1.0);
  }
}

pub fn clear_cache_check() {
  let appdata = dirs::data_dir().unwrap_or_default().join("acord");

  if !appdata.exists() {
    std::fs::create_dir_all(&appdata).expect("Failed to create acord appdata dir!");
  }

  let cache_file = appdata.join("clear_cache");

  if cache_file.exists() {
    std::fs::remove_file(&cache_file).expect("Failed to remove clear_cache file!");
    clear_cache();
  }
}

#[tauri::command]
pub fn set_clear_cache(win: tauri::WebviewWindow) {
  let appdata = dirs::data_dir().unwrap_or_default().join("acord");

  if !appdata.exists() {
    std::fs::create_dir_all(&appdata).expect("Failed to create acord appdata dir!");
  }

  std::fs::write(appdata.join("clear_cache"), "").expect("Failed to create clear_cache file!");
  win.close().unwrap_or_default();
}

#[tauri::command]
pub fn clear_cache() {
  let webdata_dir = get_webdata_dir();

  if webdata_dir.exists() {
    log!("Deleting cache...");
    std::fs::remove_dir_all(webdata_dir).expect("Failed to remove webdata dir!");
  }
}

#[tauri::command]
pub fn window_zoom_level(win: tauri::WebviewWindow, value: Option<f64>) {
  let zoom = sanitize_zoom_level(
    value.unwrap_or(
      crate::config::get_config()
        .zoom.clone()
        .unwrap_or("1.0".to_string())
        .parse::<f64>()
        .unwrap_or(1.0),
    ),
  );

  crate::platform::window::set_zoom(win, zoom);
}

#[tauri::command]
pub fn remove_top_bar(win: tauri::WebviewWindow) {
  crate::platform::window::remove_top_bar(win);
}

pub fn set_user_agent(win: &tauri::WebviewWindow) {
  crate::platform::window::set_user_agent(win);
}

/// Ensures the window is visible regardless of being unfocused, minimized, or hidden.
#[tauri::command]
pub fn ultrashow(win: tauri::WebviewWindow) {
  win.unminimize().unwrap_or_default();
  win.show().unwrap_or_default();
  win.set_focus().unwrap_or_default();
}
