use std::sync::atomic::Ordering;

use crate::{
  functionality::tray::{set_tray_icon, TrayIcon, TRAY_STATE},
  log,
  util::{
    input_validation::{is_discord_snowflake, validate_http_url, MAX_REMOTE_RESPONSE_BYTES},
    window_helpers::ultrashow,
  },
};
use tauri::Manager;

#[cfg(target_os = "windows")]
use super::helpers::is_windows_7;

#[cfg_attr(target_os = "macos", allow(dead_code))]
#[derive(serde::Deserialize, Debug, Clone)]
pub struct AdditionalData {
  guild_id: Option<String>,
  channel_id: Option<String>,
  message_id: Option<String>,
}

#[tauri::command]
pub async fn send_notification(
  win: tauri::WebviewWindow,
  title: String,
  body: String,
  icon: String,
  additional_data: Option<AdditionalData>,
) {
  if title.len() > 256 || body.len() > 4096 {
    log!("Skipping notification with oversized title or body");
    return;
  }

  // Flash the taskbar icon
  if !win.is_focused().unwrap_or(false) {
    let _ = win.request_user_attention(Some(tauri::UserAttentionType::Informational));
  }

  // Write the result of the icon
  let app = win.app_handle();
  let icon = match validate_http_url(&icon) {
    Ok(icon) => icon,
    Err(error) => {
      log!("Skipping invalid notification icon URL: {error}");
      send_notification_internal(app, title, body, String::new(), additional_data);
      return;
    }
  };
  let client = crate::util::helpers::get_http_client();
  let res = match client.get(icon).send().await {
    Ok(res) => res,
    Err(e) => {
      log!("Failed to fetch notification icon: {:?}", e);
      send_notification_internal(app, title, body, String::new(), additional_data);
      return;
    }
  };

  let content_type = res
    .headers()
    .get("content-type")
    .and_then(|value| value.to_str().ok())
    .unwrap_or_default();
  if !res.status().is_success()
    || !content_type.to_ascii_lowercase().starts_with("image/")
    || res
      .content_length()
      .is_some_and(|length| length > MAX_REMOTE_RESPONSE_BYTES)
  {
    send_notification_internal(app, title, body, String::new(), additional_data);
    return;
  }

  let icon_bytes = match res.bytes().await {
    Ok(bytes) if bytes.len() <= MAX_REMOTE_RESPONSE_BYTES as usize => bytes.to_vec(),
    _ => {
      send_notification_internal(app, title, body, String::new(), additional_data);
      return;
    }
  };

  // Then write it to a temp file
  let mut tmp_file = std::env::temp_dir();
  tmp_file.push("acord_notif_icon.png");

  if std::fs::write(&tmp_file, icon_bytes).is_err() {
    log!("Failed to create temp file for notification icon");
    send_notification_internal(app, title, body, String::new(), additional_data);
    return;
  }

  let icon_path = crate::platform::notifications::icon_path(&tmp_file);

  send_notification_internal(app, title, body, icon_path, additional_data);
}

fn send_notification_internal(
  app: &tauri::AppHandle,
  title: String,
  body: String,
  icon_path: String,
  additional_data: Option<AdditionalData>,
) {
  #[cfg(target_os = "windows")]
  {
    use crate::config::get_config;

    if !is_windows_7() && !get_config().win7_style_notifications.unwrap_or(false) {
      send_notification_internal_windows(app, title, body, icon_path, additional_data)
    } else {
      send_notification_internal_windows7(app, title, body, icon_path, additional_data)
    }
  }

  #[cfg(not(target_os = "windows"))]
  send_notification_internal_other(app, title, body, icon_path, additional_data)
}

#[cfg(not(target_os = "windows"))]
fn send_notification_internal_other(
  app: &tauri::AppHandle,
  title: String,
  body: String,
  _icon: String,
  _additional_data: Option<AdditionalData>,
) {
  use notify_rust::{Notification, Timeout};

  #[cfg(target_os = "linux")]
  let win = app.get_webview_window("main");

  #[cfg(not(target_os = "linux"))]
  let _ = app;

  match Notification::new()
    .summary(&title)
    .body(&body)
    .icon("acord")
    .timeout(Timeout::Milliseconds(5000))
    .action("default", "default")
    .show()
  {
    #[cfg(target_os = "linux")]
    Ok(n) => {
      #[cfg(target_os = "linux")]
      std::thread::spawn(move || {
        n.wait_for_action(|action| {
          if action == "default" {
            if let Some(win) = &win {
              open_notification_data(win, _additional_data);
            }
          }
        })
      });
    }
    #[cfg(not(target_os = "linux"))]
    Ok(_) => {}
    Err(e) => log!("Failed to send notification: {:?}", e),
  };
}

#[cfg(target_os = "windows")]
fn send_notification_internal_windows(
  app: &tauri::AppHandle,
  title: String,
  body: String,
  icon: String,
  additional_data: Option<AdditionalData>,
) {
  use std::path::Path;
  use tauri_winrt_notification::{IconCrop, Toast};

  let win = app.get_webview_window("main");

  let mut toast = Toast::new(&app.config().identifier)
    .icon(Path::new(&icon), IconCrop::Circular, "")
    .title(title.as_str())
    .text2(body.as_str())
    .sound(None);

  if additional_data.is_some() {
    toast = toast.on_activated({
      let additional_data = additional_data.clone();

      move |_s| {
        if let (Some(win), Some(data)) = (&win, &additional_data) {
          open_notification_data(win, Some(data.clone()));
        }
        Ok(())
      }
    });
  }

  toast
    .show()
    .unwrap_or_else(|e| log!("Failed to send notification: {:?}", e));
}

#[cfg(target_os = "windows")]
fn send_notification_internal_windows7(
  app: &tauri::AppHandle,
  title: String,
  body: String,
  icon: String,
  _additional_data: Option<AdditionalData>,
) {
  use std::path::Path;
  use win7_notifications::Notification;

  let icon = tauri::image::Image::from_path(Path::new(&icon));
  let mut notification = Notification::new();

  notification
    .appname(&app.package_info().name)
    .summary(&title)
    .body(&body);

  if let Ok(icon) = icon {
    notification.icon(icon.rgba().to_vec(), icon.width(), icon.height());
  }

  notification
    .show()
    .unwrap_or_else(|e| log!("Failed to send notification: {:?}", e));
}

#[tauri::command]
pub fn notification_count(window: tauri::WebviewWindow, amount: i64) {
  log!("Setting notification count: {}", amount);

  crate::platform::notifications::set_badge(&window, amount);

  // If the tray state is unread or default,
  if TrayIcon::from_usize(TRAY_STATE.load(Ordering::Relaxed)).is_overwrite() {
    let state = if amount == 0 { "default" } else { "unread" };
    set_tray_icon(window.app_handle().to_owned(), state.to_string());
  }
}

#[cfg_attr(target_os = "macos", allow(dead_code))]
pub fn open_notification_data(win: &tauri::WebviewWindow, additional_data: Option<AdditionalData>) {
  ultrashow(win.clone());

  // Navigate to the guild/channel/message if provided
  if let Some(data) = &additional_data {
    let guild_id = data
      .guild_id
      .as_deref()
      .filter(|id| is_discord_snowflake(id))
      .unwrap_or_default();
    let channel_id = data
      .channel_id
      .as_deref()
      .filter(|id| is_discord_snowflake(id))
      .unwrap_or_default();
    let message_id = data
      .message_id
      .as_deref()
      .filter(|id| is_discord_snowflake(id))
      .unwrap_or_default();

    let url = if !guild_id.is_empty() && !channel_id.is_empty() && !message_id.is_empty() {
      format!("/channels/{}/{}/{}", guild_id, channel_id, message_id)
    } else if !guild_id.is_empty() && !channel_id.is_empty() {
      format!("/channels/{}/{}", guild_id, channel_id)
    } else if !channel_id.is_empty() && !message_id.is_empty() && guild_id.is_empty() {
      format!("/channels/@me/{}/{}", channel_id, message_id)
    } else if !guild_id.is_empty() {
      format!("/channels/{}", guild_id)
    } else {
      String::new()
    };

    if !url.is_empty() {
      let url = serde_json::to_string(&url).unwrap_or_else(|_| "\"\"".to_string());
      win
        .eval(format!(
          r#"
      history.pushState(null, "", {url});
      window.dispatchEvent(new PopStateEvent("popstate"));
      "#
        ))
        .unwrap_or_default();
    }
  }
}
