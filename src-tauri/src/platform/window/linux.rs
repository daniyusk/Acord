pub fn set_user_agent(window: &tauri::WebviewWindow) {
  use tauri::Manager;
  use webkit2gtk::{SettingsExt, WebViewExt};

  window
    .with_webview(|webview| {
      let webview = webview.inner();
      let settings = webview.settings().unwrap();
      settings.set_user_agent(Some(&user_agent()));
    })
    .unwrap_or_else(|error| crate::log!("Failed to set user-agent: {error:?}"));
}

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

fn user_agent() -> String {
  "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36".to_string()
}
