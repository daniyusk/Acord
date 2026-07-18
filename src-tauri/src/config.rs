use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use reqwest::Url;

use crate::{
  functionality::keyboard::{validate_keybinds, KeyStruct, KeybindAction},
  log,
  util::paths::{get_config_file, validate_profile_name},
};

const MAX_THEMES: usize = 64;
const MAX_CLIENT_MODS: usize = 3;
const MAX_SETTING_STRING_BYTES: usize = 2048;

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
  /// Deprecated
  pub theme: Option<String>,

  pub themes: Option<Vec<String>>,
  pub zoom: Option<String>,
  pub client_type: Option<String>,
  pub sys_tray: Option<bool>,
  pub push_to_talk: Option<bool>,
  pub push_to_talk_keys: Option<Vec<String>>,
  pub cache_css: Option<bool>,
  pub use_native_titlebar: Option<bool>,
  pub start_maximized: Option<bool>,
  pub profile: Option<String>,
  pub streamer_mode_detection: Option<bool>,
  pub rpc_server: Option<bool>,
  pub open_on_startup: Option<bool>,
  pub startup_minimized: Option<bool>,
  pub autoupdate: Option<bool>,
  pub update_notify: Option<bool>,
  pub desktop_notifications: Option<bool>,
  pub auto_clear_cache: Option<bool>,
  pub multi_instance: Option<bool>,
  pub disable_hardware_accel: Option<bool>,
  pub blur: Option<String>,
  pub blur_css: Option<bool>,
  pub client_mods: Option<Vec<String>>,
  pub unread_badge: Option<bool>,
  pub client_plugins: Option<bool>,
  pub tray_icon_enabled: Option<bool>,
  pub proxy_uri: Option<String>,

  pub keybinds: Option<HashMap<KeybindAction, Vec<KeyStruct>>>,
  pub keybinds_enabled: Option<bool>,

  pub win7_style_notifications: Option<bool>,

  // RPC-specific options
  pub rpc_process_scanner: Option<bool>,
  pub rpc_ipc_connector: Option<bool>,
  pub rpc_websocket_connector: Option<bool>,
  pub rpc_secondary_events: Option<bool>,
}

impl Config {
  pub fn default() -> Self {
    Config {
      // Deprecated
      theme: Option::from("none".to_string()),

      themes: Option::from(vec!["none".to_string()]),
      zoom: Option::from("1.0".to_string()),
      client_type: Option::from("default".to_string()),
      sys_tray: Option::from(false),
      push_to_talk: Option::from(false),
      push_to_talk_keys: Option::from(vec!["RControl".to_string()]),
      cache_css: Option::from(false),
      use_native_titlebar: Option::from(false),
      start_maximized: Option::from(false),
      profile: Option::from("default".to_string()),
      streamer_mode_detection: Option::from(false),
      rpc_server: Option::from(false),
      open_on_startup: Option::from(false),
      startup_minimized: Option::from(false),
      autoupdate: Option::from(false),
      update_notify: Option::from(true),
      desktop_notifications: Option::from(false),
      auto_clear_cache: Option::from(false),
      multi_instance: Option::from(false),
      disable_hardware_accel: Option::from(false),
      blur: Option::from("none".to_string()),
      blur_css: Option::from(true),
      client_mods: Option::from(vec![]),
      unread_badge: Option::from(true),
      client_plugins: Option::from(false),
      tray_icon_enabled: Option::from(true),
      proxy_uri: Option::from("".to_string()),

      keybinds: Option::from(HashMap::new()),
      keybinds_enabled: Option::from(true),

      win7_style_notifications: Option::from(false),

      // RPC-specific options
      rpc_process_scanner: Option::from(true),
      rpc_ipc_connector: Option::from(true),
      rpc_websocket_connector: Option::from(true),
      rpc_secondary_events: Option::from(true),
    }
  }

  fn merge(self, other: Self) -> Self {
    Self {
      theme: other.theme.or(self.theme.clone()),
      themes: other.themes.or(self.themes.clone()),
      zoom: other.zoom.or(self.zoom.clone()),
      client_type: other.client_type.or(self.client_type.clone()),
      sys_tray: other.sys_tray.or(self.sys_tray),
      push_to_talk: other.push_to_talk.or(self.push_to_talk),
      push_to_talk_keys: other.push_to_talk_keys.or(self.push_to_talk_keys.clone()),
      cache_css: other.cache_css.or(self.cache_css),
      use_native_titlebar: other.use_native_titlebar.or(self.use_native_titlebar),
      start_maximized: other.start_maximized.or(self.start_maximized),
      profile: other.profile.or(self.profile.clone()),
      streamer_mode_detection: other
        .streamer_mode_detection
        .or(self.streamer_mode_detection),
      rpc_server: other.rpc_server.or(self.rpc_server),
      open_on_startup: other.open_on_startup.or(self.open_on_startup),
      startup_minimized: other.startup_minimized.or(self.startup_minimized),
      autoupdate: other.autoupdate.or(self.autoupdate),
      update_notify: other.update_notify.or(self.update_notify),
      desktop_notifications: other.desktop_notifications.or(self.desktop_notifications),
      auto_clear_cache: other.auto_clear_cache.or(self.auto_clear_cache),
      multi_instance: other.multi_instance.or(self.multi_instance),
      disable_hardware_accel: other.disable_hardware_accel.or(self.disable_hardware_accel),
      blur: other.blur.or(self.blur.clone()),
      blur_css: other.blur_css.or(self.blur_css),
      client_mods: other.client_mods.or(self.client_mods.clone()),
      unread_badge: other.unread_badge.or(self.unread_badge),
      client_plugins: other.client_plugins.or(self.client_plugins),
      tray_icon_enabled: other.tray_icon_enabled.or(self.tray_icon_enabled),
      proxy_uri: other.proxy_uri.or(self.proxy_uri.clone()),

      keybinds: other.keybinds.or(self.keybinds.clone()),
      keybinds_enabled: other.keybinds_enabled.or(self.keybinds_enabled),

      win7_style_notifications: other
        .win7_style_notifications
        .or(self.win7_style_notifications),

      rpc_process_scanner: other.rpc_process_scanner.or(self.rpc_process_scanner),
      rpc_ipc_connector: other.rpc_ipc_connector.or(self.rpc_ipc_connector),
      rpc_websocket_connector: other
        .rpc_websocket_connector
        .or(self.rpc_websocket_connector),
      rpc_secondary_events: other.rpc_secondary_events.or(self.rpc_secondary_events),
    }
  }

  pub fn init() {
    get_config_file();
  }

  pub fn validate(&self) -> Result<(), String> {
    if let Some(profile) = self.profile.as_deref() {
      validate_profile_name(profile)?;
    }

    if let Some(themes) = &self.themes {
      validate_setting_list("Theme list", themes, MAX_THEMES)?;
    }

    if let Some(client_mods) = &self.client_mods {
      validate_setting_list("Client mod list", client_mods, MAX_CLIENT_MODS)?;
    }

    if let Some(zoom) = self.zoom.as_deref() {
      let zoom = zoom
        .parse::<f64>()
        .map_err(|_| "Zoom must be a number".to_string())?;
      if !zoom.is_finite() || !(0.25..=5.0).contains(&zoom) {
        return Err("Zoom must be between 0.25 and 5.0".to_string());
      }
    }

    if let Some(proxy_uri) = self.proxy_uri.as_deref().filter(|uri| !uri.is_empty()) {
      let proxy = Url::parse(proxy_uri).map_err(|error| format!("Invalid proxy URL: {error}"))?;
      if !matches!(proxy.scheme(), "http" | "https") || proxy.host_str().is_none() {
        return Err("Proxy URL must use HTTP(S) and include a host".to_string());
      }
    }

    if let Some(keybinds) = &self.keybinds {
      validate_keybinds(keybinds)?;
    }

    Ok(())
  }

  pub fn from_file(path: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
    let config_str = fs::read_to_string(path)?;
    let config_str = config_str.as_str();

    match serde_json::from_str::<Config>(config_str) {
      Ok(config) => {
        let config = Self::default().merge(config);
        Ok(config)
      }
      Err(e) => {
        log!("Failed to parse config, using default config!");
        log!("Error: {}", e);

        Ok(Self::default())
      }
    }
  }

  pub fn to_file(&self, path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    fs::write(path, serde_json::to_string(&self)?)?;

    Ok(())
  }

  pub fn from_file_str(path: PathBuf) -> Result<String, Box<dyn std::error::Error>> {
    let config = Config::from_file(path)?;
    let config_str = serde_json::to_string(&config)?;

    Ok(config_str)
  }

  pub fn from_str(contents: impl AsRef<str>) -> Result<Self, Box<dyn std::error::Error>> {
    match serde_json::from_str::<Config>(contents.as_ref()) {
      Ok(config) => {
        let config = Self::default().merge(config);
        Ok(config)
      }
      Err(e) => {
        log!("Failed to parse config, using default config!");
        log!("Error: {}", e);

        Ok(Self::default())
      }
    }
  }
}

#[tauri::command]
pub fn read_config_file() -> String {
  Config::from_file_str(get_config_file()).expect("Config does not exist!")
}

#[tauri::command]
pub fn write_config_file(contents: String) -> Result<(), String> {
  let config = Config::from_str(&contents).map_err(|error| error.to_string())?;
  config.validate()?;
  config
    .to_file(get_config_file())
    .map_err(|error| format!("Error writing config: {error}"))
}

fn validate_setting_list(name: &str, values: &[String], max_entries: usize) -> Result<(), String> {
  if values.len() > max_entries {
    return Err(format!(
      "{name} cannot contain more than {max_entries} entries"
    ));
  }

  if values.iter().any(|value| {
    value.is_empty()
      || value.len() > MAX_SETTING_STRING_BYTES
      || value.chars().any(char::is_control)
  }) {
    return Err(format!("{name} contains an invalid value"));
  }

  Ok(())
}

#[tauri::command]
pub fn default_config() -> Config {
  Config::default()
}

#[tauri::command]
pub fn get_config() -> Config {
  let config_str = read_config_file();

  Config::from_str(&config_str).expect("Error parsing config!")
}

#[tauri::command]
pub fn set_config(config: Config) -> Result<(), String> {
  config.validate()?;

  let config_str = serde_json::to_string(&config)
    .map_err(|error| format!("Failed to serialize config: {error}"))?;

  write_config_file(config_str)
}

#[cfg(test)]
mod tests {
  use std::collections::HashMap;

  use super::{Config, KeyStruct, KeybindAction};

  #[test]
  fn accepts_the_default_configuration() {
    assert!(Config::default().validate().is_ok());
  }

  #[test]
  fn rejects_configurations_with_unsafe_profile_names() {
    let mut config = Config::default();
    config.profile = Some("../outside".to_string());

    assert!(config.validate().is_err());
  }

  #[test]
  fn rejects_unsafe_proxy_and_oversized_client_mod_lists() {
    let mut config = Config::default();
    config.proxy_uri = Some("file:///tmp/proxy".to_string());
    assert!(config.validate().is_err());

    config.proxy_uri = None;
    config.client_mods = Some(vec!["Shelter".to_string(); 4]);
    assert!(config.validate().is_err());
  }

  #[test]
  fn rejects_invalid_zoom_theme_and_keybind_values() {
    let mut config = Config::default();
    config.zoom = Some("not-a-number".to_string());
    assert!(config.validate().is_err());

    config.zoom = Some("6.0".to_string());
    assert!(config.validate().is_err());

    config = Config::default();
    config.themes = Some(vec!["bad\ntheme".to_string()]);
    assert!(config.validate().is_err());

    config = Config::default();
    let mut keybinds = HashMap::new();
    keybinds.insert(
      KeybindAction::PushToTalk,
      vec![KeyStruct {
        name: "Ctrl".to_string(),
        code: "Control\nLeft".to_string(),
      }],
    );
    config.keybinds = Some(keybinds);
    assert!(config.validate().is_err());
  }
}
