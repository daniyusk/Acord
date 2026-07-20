pub fn set_user_agent(_window: &tauri::WebviewWindow) {}

pub fn set_zoom(window: tauri::WebviewWindow, zoom: f64) {
  window
    .eval(format!("document.body.style.zoom = '{zoom}'"))
    .expect("Failed to set zoom level!");
}

pub fn remove_top_bar(window: tauri::WebviewWindow) {
  window.set_decorations(false).unwrap_or(());
}

#[cfg(feature = "blur")]
pub fn available_blurs() -> Vec<&'static str> {
  vec!["none", "transparent"]
}

#[cfg(feature = "blur")]
pub fn apply_effect(_window: tauri::WebviewWindow, _effect: &str) {}
