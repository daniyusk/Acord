pub fn configure_before_creation(disable_hardware_accel: bool) {
  if disable_hardware_accel {
    let existing_args = std::env::var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS").unwrap_or_default();
    unsafe {
      std::env::set_var(
        "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
        format!("{existing_args} --disable-gpu"),
      );
    };
  }

  let browser_args = std::env::var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS").unwrap_or_default();
  let new_args = crate::args::get_webview_args();

  unsafe {
    std::env::set_var(
      "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
      format!("{browser_args} {new_args}"),
    );
  };

  crate::log!("Running with the following WebView2 arguments: {browser_args} {new_args}");
}

pub fn configure_after_creation(_window: &tauri::WebviewWindow) {}
