fn main() {
  tauri_build::try_build(
    tauri_build::Attributes::new()
      .app_manifest(tauri_build::AppManifest::new().commands(&[
        // Keep this list in sync with the commands registered in `main.rs`.
        // Commands omitted here cannot be invoked by a webview.
        "should_disable_plugins",
        "git_hash",
        "extension_injected",
        "minimize",
        "toggle_maximize",
        #[cfg(not(target_os = "macos"))]
        "set_decorations",
        "close",
        "clear_css_cache",
        "localize_imports",
        "localize_all_js",
        "get_index",
        "get_extra_css",
        "notification_count",
        "send_notification",
        "load_plugins",
        "get_plugin_list",
        "toggle_plugin",
        "toggle_preload",
        "get_plugin_import_urls",
        "available_mods",
        "load_mods_css",
        "get_profile_list",
        "get_current_profile_folder",
        "create_profile",
        "delete_profile",
        "do_update",
        "update_check",
        #[cfg(all(feature = "rpc", not(target_os = "macos")))]
        "get_windows",
        #[cfg(all(feature = "rpc", not(target_os = "macos")))]
        "get_local_detectables",
        #[cfg(all(feature = "hotkeys", not(target_os = "macos")))]
        "get_keybinds",
        #[cfg(all(feature = "hotkeys", not(target_os = "macos")))]
        "set_keybinds",
        #[cfg(all(feature = "hotkeys", not(target_os = "macos")))]
        "set_keybind",
        #[cfg(all(
          feature = "hotkeys",
          not(target_os = "macos"),
          not(target_os = "linux")
        ))]
        "trigger_keys_pressed",
        "set_tray_icon",
        "get_injection_js",
        "get_config",
        "set_config",
        "read_config_file",
        "write_config_file",
        "default_config",
        "restart_in_safemode",
        "get_themes",
        "get_theme_names",
        "get_enabled_themes",
        "theme_from_link",
        "get_platform",
        "open_themes",
        "open_plugins",
        "open_extensions",
        "fetch_image",
        #[cfg(feature = "blur")]
        "available_blurs",
        #[cfg(feature = "blur")]
        "apply_effect",
        "remove_top_bar",
        "set_clear_cache",
        "ultrashow",
        "window_zoom_level",
        "get_os_accent",
      ])),
  )
  .expect("failed to build the Tauri application manifest");
}
