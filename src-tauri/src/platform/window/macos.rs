pub fn set_user_agent(window: &tauri::WebviewWindow) {
  use objc2_foundation::NSString;
  use objc2_web_kit::WKWebView;
  use tauri::Manager;

  window
    .with_webview(|webview| unsafe {
      let webview: &WKWebView = &*webview.inner().cast();
      let user_agent = NSString::from_str(&user_agent());
      webview.setCustomUserAgent(Some(&user_agent));
    })
    .unwrap_or_else(|error| crate::log!("Failed to set user-agent: {error:?}"));
}

pub fn set_zoom(window: tauri::WebviewWindow, zoom: f64) {
  window
    .eval(format!("document.body.style.zoom = '{zoom}'"))
    .expect("Failed to set zoom level!");
}

pub fn remove_top_bar(_window: tauri::WebviewWindow) {}

#[cfg(feature = "blur")]
pub fn available_blurs() -> Vec<&'static str> {
  vec!["none", "vibrancy", "transparent"]
}

#[cfg(feature = "blur")]
pub fn apply_effect(window: tauri::WebviewWindow, effect: &str) {
  use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial};

  if effect == "vibrancy" {
    apply_vibrancy(window, NSVisualEffectMaterial::HudWindow, None, None).unwrap_or_default();
  }
}

fn user_agent() -> String {
  "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36".to_string()
}
