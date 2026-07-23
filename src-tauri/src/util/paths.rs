use std::{
  fs,
  path::{Component, Path, PathBuf},
};

use tauri::path::BaseDirectory;
use tauri::Manager;

use crate::config::{default_config, get_config};
use crate::{args, log};

fn create_if_not_exists(path: &PathBuf) {
  if fs::metadata(path).is_err() {
    match fs::create_dir_all(path) {
      Ok(()) => (),
      Err(e) => {
        log!("Error creating dir: {}", e);
      }
    };
  }
}

pub fn is_portable() -> bool {
  let current_exe = std::env::current_exe().unwrap_or_default();
  let portable_signifier = current_exe.parent().unwrap().join(".portable");

  fs::metadata(portable_signifier).is_ok()
}

pub fn get_config_dir() -> PathBuf {
  // First check for a local config file
  let current_exe = std::env::current_exe().unwrap_or_default();
  let local_config_dir = current_exe.parent().unwrap().to_path_buf();

  if is_portable() {
    return local_config_dir;
  }

  let config_dir = crate::platform::paths::config_root();

  create_if_not_exists(&config_dir);

  config_dir
}

pub fn get_config_file() -> PathBuf {
  let config_dir = get_config_dir();
  let config_file = config_dir.join("config.json");

  // Write default config if it doesn't exist
  if fs::metadata(&config_file).is_err() {
    fs::write(
      &config_file,
      serde_json::to_string_pretty(&default_config()).unwrap_or_default(),
    )
    .unwrap_or(());
  }

  config_file
}

pub fn config_is_local() -> bool {
  let current_exe = std::env::current_exe().unwrap_or_default();
  let local_config_dir = current_exe.parent().unwrap().join("config.json");

  fs::metadata(local_config_dir).is_ok()
}

pub fn get_plugin_dir() -> std::path::PathBuf {
  // First check for a local plugin dir
  let current_exe = std::env::current_exe().unwrap_or_default();
  let local_plugin_dir = current_exe.parent().unwrap().join("plugins");

  if is_portable() {
    // Create dir if it doesn't exist
    create_if_not_exists(&local_plugin_dir);

    return local_plugin_dir;
  }

  let plugin_dir = crate::platform::paths::user_content_root().join("plugins");

  create_if_not_exists(&plugin_dir);

  plugin_dir
}

pub fn get_theme_dir() -> std::path::PathBuf {
  // First see if there is a local theme dir
  let current_exe = std::env::current_exe().unwrap_or_default();
  let local_theme_dir = current_exe.parent().unwrap().join("themes");

  if is_portable() {
    // Create dir if it doesn't exist
    create_if_not_exists(&local_theme_dir);

    return local_theme_dir;
  }

  let theme_dir = crate::platform::paths::user_content_root().join("themes");

  create_if_not_exists(&theme_dir);

  // Also create theme cache dir
  let cache_dir = theme_dir.join("cache");

  create_if_not_exists(&cache_dir);

  theme_dir
}

pub fn get_extensions_dir() -> PathBuf {
  let current_exe = std::env::current_exe().unwrap_or_default();

  // Check for local/portable file paths
  if is_portable() {
    let extensions_folder = current_exe.parent().unwrap().join("extensions");

    create_if_not_exists(&extensions_folder);

    return extensions_folder;
  }

  let extensions_dir = crate::platform::paths::user_content_root().join("extensions");

  create_if_not_exists(&extensions_dir);

  extensions_dir
}

#[cfg(target_os = "windows")]
pub fn get_main_extension_path() -> PathBuf {
  let current_exe = std::env::current_exe().unwrap_or_default();

  // Check for local/portable file paths
  if is_portable() {
    let extension_folder = current_exe.parent().unwrap().join("extension");

    create_if_not_exists(&extension_folder);

    return extension_folder;
  }

  let extension_dir = crate::platform::paths::config_root().join("extension");

  create_if_not_exists(&extension_dir);

  extension_dir
}

pub fn profiles_dir() -> PathBuf {
  let current_exe = std::env::current_exe().unwrap_or_default();

  // Check for local/portable file paths
  if is_portable() {
    let profile_folder = current_exe.parent().unwrap().join("profiles");

    create_if_not_exists(&profile_folder);

    return profile_folder;
  }

  // This is created automatically
  dirs::data_dir()
    .unwrap_or_default()
    .join("acord")
    .join("profiles")
}

pub fn validate_profile_name(name: &str) -> Result<(), String> {
  if name.is_empty() {
    return Err("Profile name cannot be empty".to_string());
  }

  if name.len() > 255 {
    return Err("Profile name cannot exceed 255 bytes".to_string());
  }

  if name.chars().any(|character| {
    character.is_control()
      || matches!(
        character,
        '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'
      )
  }) {
    return Err("Profile name contains an invalid path character".to_string());
  }

  let mut components = Path::new(name).components();
  if !matches!(components.next(), Some(Component::Normal(_))) || components.next().is_some() {
    return Err("Profile name must be a single directory name".to_string());
  }

  Ok(())
}

pub fn profile_path(profiles_dir: &Path, name: &str) -> Result<PathBuf, String> {
  validate_profile_name(name)?;

  Ok(profiles_dir.join(name))
}

pub fn get_webdata_dir() -> PathBuf {
  // Grab from args first, it should take precedence
  let profile = match args::get_profile() {
    Some(p) => p,
    None => {
      let cfg = get_config();
      cfg.profile.unwrap_or("default".to_string())
    }
  };
  let profiles = profiles_dir();
  let profile_dir = match profile_path(&profiles, &profile) {
    Ok(profile_dir) => profile_dir,
    Err(error) => {
      log!("Ignoring invalid profile name while resolving web data: {error}");
      profiles.join("default")
    }
  };
  let dir = profile_dir.join("webdata");

  // as a precaution, ensure the directory exists
  create_if_not_exists(&dir);

  dir
}

pub fn updater_dir(win: &tauri::WebviewWindow) -> PathBuf {
  let current_exe = std::env::current_exe().unwrap_or_default();

  if is_portable() {
    // This is a portable install, so we can use the local dir
    return current_exe.parent().unwrap().join("updater");
  }

  win
    .app_handle()
    .path()
    .resolve(PathBuf::from("updater"), BaseDirectory::Resource)
    .unwrap_or_default()
}

#[cfg(feature = "rpc")]
#[cfg(not(target_os = "macos"))]
pub fn custom_detectables_path() -> PathBuf {
  let current_exe = std::env::current_exe().unwrap_or_default();

  if is_portable() {
    // This is a portable install, so we can use the local dir
    return current_exe.parent().unwrap().join("detectables.json");
  }

  crate::platform::paths::config_root().join("detectables.json")
}

pub fn log_file_path() -> PathBuf {
  let current_exe = std::env::current_exe().unwrap_or_default();

  if is_portable() {
    // This is a portable install, so we can use the local dir
    return current_exe
      .parent()
      .unwrap()
      .join("logs")
      .join("latest.log");
  }

  crate::platform::paths::config_root()
    .join("logs")
    .join("latest.log")
}

#[cfg(test)]
mod tests {
  use std::path::Path;

  use super::{profile_path, validate_profile_name};

  #[test]
  fn allows_single_directory_profile_names() {
    for name in ["default", "Profile 1", "Jos\u{00e9}", ".private"] {
      assert!(
        validate_profile_name(name).is_ok(),
        "{name} should be valid"
      );
    }
  }

  #[test]
  fn rejects_path_traversal_and_platform_separators() {
    let oversized = "a".repeat(256);
    for name in [
      "",
      ".",
      "..",
      "../outside",
      "..\\outside",
      "nested/profile",
      "nested\\profile",
      "C:\\profiles",
      "/tmp/profile",
      "profile\0name",
      oversized.as_str(),
    ] {
      assert!(
        validate_profile_name(name).is_err(),
        "{name:?} should be rejected"
      );
    }
  }

  #[test]
  fn joins_only_valid_profile_names() {
    let root = Path::new("profiles");

    assert_eq!(profile_path(root, "work").unwrap(), root.join("work"));
    assert!(profile_path(root, "../outside").is_err());
  }

  #[test]
  fn accepts_profile_name_at_the_length_limit() {
    assert!(validate_profile_name(&"a".repeat(255)).is_ok());
  }
}
