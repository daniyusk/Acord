use std::fs;

use crate::{
  config::get_config,
  util::{
    input_validation::{
      normalize_css_file_name, validate_http_url, validate_payload_size, MAX_CSS_BYTES,
    },
    paths::get_theme_dir,
  },
};

fn theme_is_enabled(name: String) -> bool {
  let config = get_config();
  config.themes.unwrap_or_default().contains(&name)
}

#[tauri::command]
pub fn get_themes() -> Result<String, String> {
  let themes = get_theme_dir();
  let mut all_contents = String::new();

  for entry in fs::read_dir(themes).map_err(|e| format!("Error reading theme directory: {e}"))? {
    let entry = entry.map_err(|e| format!("Error reading theme directory: {e}"))?;
    let file_name = entry
      .file_name()
      .to_str()
      .map(|name| name.to_string())
      .filter(|name| name != "cache" && name != ".ds_store")
      .unwrap_or_default();

    if file_name.ends_with(".css") && theme_is_enabled(file_name) {
      all_contents.push_str(
        fs::read_to_string(entry.path())
          .unwrap_or_default()
          .as_str(),
      );
    }
  }

  Ok(all_contents)
}

#[tauri::command]
pub fn get_theme_names() -> Result<Vec<String>, String> {
  let themes_dir = get_theme_dir();
  let theme_folders =
    fs::read_dir(themes_dir).map_err(|e| format!("Error reading theme directory: {e}"))?;
  let names = theme_folders
    .filter_map(|entry| {
      entry.ok().and_then(|file| {
        file
          .file_name()
          .to_str()
          .map(|name| name.to_string())
          .filter(|name| {
            let lowercase = name.to_lowercase();
            lowercase != "cache" && lowercase != ".ds_store"
          })
      })
    })
    .map(|folder_name| format!("{folder_name:?}"))
    .collect();

  Ok(names)
}

#[tauri::command]
pub fn get_enabled_themes() -> Result<Vec<String>, String> {
  let config = get_config();
  Ok(config.themes.unwrap_or_default())
}

#[tauri::command]
pub async fn theme_from_link(link: String, filename: Option<String>) -> Result<String, String> {
  let link = validate_http_url(&link)?;
  let name = filename.unwrap_or_else(|| {
    link
      .path_segments()
      .and_then(|mut segments| segments.next_back())
      .filter(|segment| !segment.is_empty())
      .unwrap_or("unnamed")
      .to_string()
  });
  let filename = normalize_css_file_name(&name)?;

  let client = reqwest::Client::builder()
    .redirect(reqwest::redirect::Policy::none())
    .build()
    .map_err(|error| format!("Failed to create HTTP client: {error}"))?;
  let response = client
    .get(link)
    .send()
    .await
    .map_err(|error| format!("Failed to fetch theme: {error}"))?;

  if !response.status().is_success() {
    return Err(format!(
      "Theme request failed with status {}",
      response.status()
    ));
  }

  if response
    .content_length()
    .is_some_and(|length| length > MAX_CSS_BYTES as u64)
  {
    return Err("Theme is too large".to_string());
  }

  let theme = response
    .text()
    .await
    .map_err(|error| format!("Failed to read theme: {error}"))?;
  validate_payload_size(&theme, MAX_CSS_BYTES, "Theme")?;

  fs::write(get_theme_dir().join(&filename), theme)
    .map_err(|error| format!("Failed to write theme: {error}"))?;

  Ok(filename)
}
