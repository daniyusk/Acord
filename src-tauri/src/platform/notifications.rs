pub(crate) fn icon_path(path: &std::path::Path) -> String {
  imp::icon_path(path)
}

pub(crate) fn set_badge(window: &tauri::WebviewWindow, amount: i64) {
  imp::set_badge(window, amount)
}

#[cfg(target_os = "windows")]
mod imp {
  pub(super) fn icon_path(path: &std::path::Path) -> String {
    path.to_str().unwrap_or_default().replace('\\', "/")
  }

  pub(super) fn set_badge(window: &tauri::WebviewWindow, amount: i64) {
    if amount == 0 {
      window.set_overlay_icon(None).unwrap_or_default();
      return;
    }

    use include_flate::flate;
    use tauri::image::Image;

    flate!(static ICO_SOME: [u8] from "./icons/notifications/some.png");
    flate!(static ICO_1: [u8] from "./icons/notifications/1.png");
    flate!(static ICO_2: [u8] from "./icons/notifications/2.png");
    flate!(static ICO_3: [u8] from "./icons/notifications/3.png");
    flate!(static ICO_4: [u8] from "./icons/notifications/4.png");
    flate!(static ICO_5: [u8] from "./icons/notifications/5.png");
    flate!(static ICO_6: [u8] from "./icons/notifications/6.png");
    flate!(static ICO_7: [u8] from "./icons/notifications/7.png");
    flate!(static ICO_8: [u8] from "./icons/notifications/8.png");
    flate!(static ICO_9: [u8] from "./icons/notifications/9.png");

    let icon = match amount {
      -1 => ICO_SOME.as_ref(),
      1 => ICO_1.as_ref(),
      2 => ICO_2.as_ref(),
      3 => ICO_3.as_ref(),
      4 => ICO_4.as_ref(),
      5 => ICO_5.as_ref(),
      6 => ICO_6.as_ref(),
      7 => ICO_7.as_ref(),
      8 => ICO_8.as_ref(),
      9 => ICO_9.as_ref(),
      _ => ICO_9.as_ref(),
    };

    match Image::from_bytes(icon) {
      Ok(icon) => window.set_overlay_icon(Some(icon)).unwrap_or_default(),
      Err(error) => {
        crate::log!("Failed to convert notification icon: {error:?}");
        window.set_overlay_icon(None).unwrap_or_default();
      }
    }
  }
}

#[cfg(target_os = "linux")]
mod imp {
  pub(super) fn icon_path(path: &std::path::Path) -> String {
    format!(
      "file://{}",
      path.to_str().unwrap_or_default().replace('\\', "/")
    )
  }

  pub(super) fn set_badge(window: &tauri::WebviewWindow, amount: i64) {
    window
      .set_badge_count(if amount <= 0 { None } else { Some(amount) })
      .unwrap_or_default();
  }
}

#[cfg(target_os = "macos")]
mod imp {
  pub(super) fn icon_path(path: &std::path::Path) -> String {
    format!(
      "file://{}",
      path.to_str().unwrap_or_default().replace('\\', "/")
    )
  }

  pub(super) fn set_badge(_window: &tauri::WebviewWindow, amount: i64) {
    use objc2_app_kit::NSApp;
    use objc2_foundation::{MainThreadMarker, NSString};

    let label = if amount > 0 {
      Some(NSString::from_str(&format!("{amount}")))
    } else if amount == -1 {
      Some(NSString::from_str("●"))
    } else {
      None
    };

    if let Some(thread) = MainThreadMarker::new() {
      unsafe {
        let app = NSApp(thread);
        let dock_tile = app.dockTile();
        dock_tile.setBadgeLabel(label.as_deref());
        dock_tile.display();
      }
    } else {
      crate::log!("Failed to mark main thread!");
    }
  }
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
mod imp {
  pub(super) fn icon_path(path: &std::path::Path) -> String {
    format!(
      "file://{}",
      path.to_str().unwrap_or_default().replace('\\', "/")
    )
  }

  pub(super) fn set_badge(_window: &tauri::WebviewWindow, _amount: i64) {}
}
