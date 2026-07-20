pub fn set_user_agent(window: &tauri::WebviewWindow) {
  use tauri::webview::PlatformWebview;
  use webview2_com::Microsoft::Web::WebView2::Win32::{ICoreWebView2Settings2, ICoreWebView2_2};
  use windows::core::{Interface, HSTRING, PWSTR};

  window
    .with_webview(|webview| unsafe {
      unsafe fn inner(webview: PlatformWebview) -> Result<(), Box<dyn std::error::Error>> {
        let webview = webview
          .controller()
          .CoreWebView2()?
          .cast::<ICoreWebView2_2>()?;
        let settings = webview.Settings()?.cast::<ICoreWebView2Settings2>()?;
        let environment = webview.Environment()?;
        let mut browser_version = PWSTR::null();

        environment.BrowserVersionString(&mut browser_version)?;
        let browser_version = browser_version
          .to_string()?
          .chars()
          .take_while(|&character| character != '.')
          .collect::<String>();

        crate::log!("Webview2 Chromium version: {browser_version}.0.0.0");
        let browser_version =
          (!browser_version.is_empty()).then(|| format!("{browser_version}.0.0.0"));
        settings.SetUserAgent(&HSTRING::from(user_agent(browser_version)))?;
        Ok(())
      }

      inner(webview).unwrap_or_else(|error| crate::log!("Failed to set user-agent: {error:?}"));
    })
    .unwrap_or_else(|error| crate::log!("Failed to set user-agent: {error:?}"));

  crate::log!("Set user agent!");
}

pub fn set_zoom(window: tauri::WebviewWindow, zoom: f64) {
  window
    .with_webview(move |webview| unsafe {
      webview.controller().SetZoomFactor(zoom).unwrap_or_default();
    })
    .unwrap_or_default();
}

pub fn remove_top_bar(window: tauri::WebviewWindow) {
  window.set_decorations(false).unwrap_or(());
}

#[cfg(feature = "blur")]
pub fn available_blurs() -> Vec<&'static str> {
  vec!["none", "blur", "acrylic", "mica", "transparent"]
}

#[cfg(feature = "blur")]
pub fn apply_effect(window: tauri::WebviewWindow, effect: &str) {
  use window_vibrancy::{apply_acrylic, apply_blur, apply_mica};

  match effect {
    "blur" => apply_blur(window, Some((18, 18, 18, 125))).unwrap_or_default(),
    "acrylic" => apply_acrylic(window, Some((18, 18, 18, 125))).unwrap_or_default(),
    "mica" => apply_mica(window, None).unwrap_or_default(),
    _ => (),
  }
}

fn user_agent(chrome_version: Option<String>) -> String {
  let chrome_version = chrome_version.unwrap_or("131.0.0.0".to_string());
  format!(
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/{chrome_version} Safari/537.36"
  )
}
