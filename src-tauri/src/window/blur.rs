#[tauri::command]
pub fn available_blurs() -> Vec<&'static str> {
  crate::platform::window::available_blurs()
}

#[tauri::command]
pub fn apply_effect(window: tauri::WebviewWindow, effect: &str) {
  crate::platform::window::apply_effect(window, effect);
}
