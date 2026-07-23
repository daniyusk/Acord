use regex::Regex;
use std::fs;

#[cfg(not(target_os = "windows"))]
use std::io::Read;

use crate::log;
use crate::util::{
  input_validation::{validate_payload_size, MAX_CSS_BYTES},
  paths::get_theme_dir,
};

#[cfg(not(target_os = "windows"))]
use crate::util::input_validation::{
  validate_file_name, validate_http_url, MAX_REMOTE_RESPONSE_BYTES,
};

#[cfg(not(target_os = "windows"))]
const MAX_CSS_IMPORTS: usize = 32;

#[tauri::command]
pub async fn clear_css_cache() {
  let cache_path = get_theme_dir().join("cache");

  if fs::metadata(&cache_path).is_ok() {
    let files = fs::read_dir(&cache_path).expect("Failed to read cache directory!");

    // Remove all files within
    for file in files.flatten() {
      fs::remove_file(file.path()).expect("Failed to remove file!");
    }
  }
}

#[cfg(not(target_os = "windows"))]
#[tauri::command]
pub fn localize_imports(win: tauri::WebviewWindow, css: String, name: String) -> String {
  use tauri::Emitter;

  use crate::config::get_config;

  if let Err(error) = validate_payload_size(&css, MAX_CSS_BYTES, "CSS") {
    log!("Skipping oversized CSS payload: {error}");
    return String::new();
  }

  let reg = Regex::new(r#"(?m)^@import url\((?:"|'|)(?:|.+?)\/\/(.+?)(?:"|'|)\);"#).unwrap();
  let mut seen_urls: Vec<String> = vec![];
  let mut new_css = css.clone();
  let cache_file_name = match validate_file_name(&name) {
    Ok(()) => Some(format!("{name}_cache.css")),
    Err(error) => {
      log!("Skipping CSS cache for invalid theme name: {error}");
      None
    }
  };

  let matches: Vec<(String, String)> = reg
    .captures_iter(&css)
    .filter_map(|groups| {
      let full_import = groups.get(0)?.as_str().to_string();
      let url = groups.get(1)?.as_str().replace(['\'', '\"'], "");
      Some((full_import, url))
    })
    .collect();

  let mut tasks = Vec::new();
  let client = match reqwest::blocking::Client::builder()
    .redirect(reqwest::redirect::Policy::none())
    .build()
  {
    Ok(client) => client,
    Err(error) => {
      log!("Failed to create HTTP client: {error}");
      return String::new();
    }
  };

  // If we need to cache CSS, first check and use cache if it exists
  if get_config().cache_css.unwrap_or(true) {
    if let Some(cache_file_name) = &cache_file_name {
      let cache_path = get_theme_dir().join("cache");

      let cache_file = cache_path.join(cache_file_name);

      if fs::metadata(&cache_file).is_ok() {
        log!("Using cached CSS for {}", name);

        // if reading to string succeeds, return that
        if let Ok(cached) = fs::read_to_string(cache_file) {
          return cached;
        }
      }
    }
  }

  for (full_import, url) in matches {

    if url.is_empty() {
      continue;
    }

    if seen_urls.contains(&url) {
      // Remove the import statement from the css
      new_css = new_css.replace(&full_import, "");
      continue;
    }

    if seen_urls.len() >= MAX_CSS_IMPORTS {
      log!("Skipping CSS imports beyond the {MAX_CSS_IMPORTS} URL limit");
      new_css = new_css.replace(&full_import, "");
      continue;
    }

    let request_url = match validate_http_url(&format!("https://{url}")) {
      Ok(url) => url,
      Err(error) => {
        log!("Skipping invalid CSS import URL: {error}");
        new_css = new_css.replace(&full_import, "");
        continue;
      }
    };

    let win_clone = win.clone(); // For use within the thread
    let client = client.clone();
    let url_clone = url.clone();
    let full_import_clone = full_import.clone();

    seen_urls.push(url.clone());

    tasks.push(std::thread::spawn(move || {
      log!("Getting: {}", &url_clone);

      let response = match client.get(request_url).send() {
        Ok(r) => r,
        Err(e) => {
          log!("Request failed: {}", e);
          log!("URL: {}", &url_clone);

          return Some((full_import_clone.clone(), String::new()));
        }
      };

      if !response.status().is_success()
        || response
          .content_length()
          .is_some_and(|length| length > MAX_CSS_BYTES as u64)
      {
        log!("Request failed: {}", response.status());
        log!("URL: {}", &url_clone);

        return Some((full_import_clone.clone(), String::new()));
      }

      let mut text = String::new();
      if response
        .take(MAX_CSS_BYTES as u64 + 1)
        .read_to_string(&mut text)
        .is_err()
        || validate_payload_size(&text, MAX_CSS_BYTES, "CSS import").is_err()
      {
        return Some((full_import_clone.clone(), String::new()));
      }

      // Emit a loading log
      win_clone
        .emit(
          "loading_log",
          format!("Processed CSS import: {}", url_clone),
        )
        .unwrap_or_default();

      Some((full_import_clone, text))
    }));
  }

  for task in tasks {
    let result = match task.join() {
      Ok(r) => r,
      Err(e) => {
        log!("Error joining thread: {:?}", e);
        continue;
      }
    };

    log!("Joining (localize_imports)...");

    if result.is_none() {
      continue;
    }

    let (url, processed) = result.unwrap();

    if new_css.len().saturating_add(processed.len()) > MAX_CSS_BYTES {
      log!("Skipping CSS import because the combined stylesheet would be too large");
      new_css = new_css.replace(url.as_str(), "");
      continue;
    }

    log!(
      "Replacing URL: {} with CSS that is {} characters long",
      url,
      processed.len()
    );

    new_css = new_css.replace(url.as_str(), processed.as_str());
  }

  // If any of this css still contains imports, we need to re-process it
  if reg.is_match(new_css.as_str()) {
    log!("Re-processing CSS imports...");
    new_css = localize_imports(win.clone(), new_css, name.clone());
  }

  win
    .emit(
      "loading_log",
      format!("Finished processing {} CSS imports", seen_urls.len()),
    )
    .unwrap_or_default();

  // Now localize images to base64 data representations
  new_css = localize_images(win.clone(), new_css);

  // If we need to cache css, do that
  if get_config().cache_css.unwrap_or(true) {
    if let Some(cache_file_name) = cache_file_name {
      let cache_path = get_theme_dir().join("cache");

      // Ensure cache path exists
      if fs::metadata(&cache_path).is_err() {
        fs::create_dir(&cache_path).expect("Failed to create cache directory!");
      }

      let cache_file = cache_path.join(cache_file_name);

      fs::write(cache_file, new_css.clone()).expect("Failed to write cache file!");
    }
  }

  new_css
}

#[cfg(target_os = "windows")]
#[tauri::command]
pub fn localize_imports(_win: tauri::WebviewWindow, css: String, _name: String) -> String {
  if let Err(error) = validate_payload_size(&css, MAX_CSS_BYTES, "CSS") {
    log!("Skipping oversized CSS payload: {error}");
    return String::new();
  }

  log!(
    "Windows no longer requires CSS imports to be localized, but it does require import shuffling!"
  );

  // We do still need to shuffle the @import statements to all be at the top
  let mut new_css = css.clone();
  let mut seen_imports: Vec<String> = vec![];

  let reg = Regex::new(r#"@import url\((?:"|'|)(?:|.+?)\/\/(.+?)(?:"|'|)\);"#).unwrap();

  for groups in reg.captures_iter(css.as_str()) {
    let full_import = groups.get(0).unwrap().as_str();
    let url = groups.get(1).unwrap().as_str().replace(['\'', '\"'], "");

    if url.is_empty() || seen_imports.contains(&url) {
      continue;
    }

    seen_imports.push(url.to_string());
    new_css = new_css.replace(&full_import, "");
  }

  // Now add all the @import statements to the top
  for import in seen_imports {
    let import = format!("@import url(\"https://{import}\");");
    new_css = format!("{import}\n{new_css}");
  }

  new_css
}

#[cfg(not(target_os = "windows"))]
pub fn localize_images(win: tauri::WebviewWindow, css: String) -> String {
  use base64::{engine::general_purpose, Engine as _};
  use tauri::Emitter;

  let img_reg = Regex::new(r#"url\((?:'|"|)(http.+?)(?:'|"|)\)"#).unwrap();
  let mut new_css = css.clone();
  let matches: Vec<String> = img_reg
    .captures_iter(&css)
    .filter_map(|groups| groups.get(1).map(|m| m.as_str().to_string()))
    .collect();

  let mut seen_urls: Vec<String> = vec![];

  // This could be pretty computationally expensive for just a count, so I should change this sometime
  let count = img_reg.captures_iter(&css).count();

  let mut tasks = Vec::new();

  // Check if the matches iter is more than 50
  // If it is, we should just skip it
  if count > 50 {
    win
      .emit(
        "loading_log",
        format!("Too many images to process ({count}), skipping...",),
      )
      .unwrap_or_default();
    return new_css;
  }

  let client = match reqwest::blocking::Client::builder()
    .redirect(reqwest::redirect::Policy::none())
    .build()
  {
    Ok(client) => client,
    Err(error) => {
      log!("Failed to create HTTP client: {error}");
      return new_css;
    }
  };

  for url in matches {
    let request_url = match validate_http_url(url) {
      Ok(url) => url,
      Err(error) => {
        log!("Skipping invalid image import URL: {error}");
        continue;
      }
    };
    let filetype = url
      .split('?')
      .next()
      .unwrap_or(url)
      .split('.')
      .next_back()
      .unwrap_or("png");

    // SVGs require the filetype to be svg+xml because they're special I guess
    let filetype = if filetype == "svg" {
      "svg+xml".to_string()
    } else {
      filetype.to_string()
    };

    // CORS allows discord media
    if url.is_empty()
            || url.contains(".css")
            || url.contains("data:image")
            || url.contains("media.discordapp")
            || url.contains("cdn.discordapp")
            || url.contains("discord.com/assets")
            // Imgur is allowed(?)
            || url.contains("i.imgur.com")
    {
      continue;
    }

    if seen_urls.contains(&url.to_string()) {
      continue;
    }

    seen_urls.push(url.clone());

    // If there are more than 50 tasks, it's safe to say that there are probably too many images
    // to process, so we should just skip it
    if tasks.len() >= 50 {
      win
        .emit(
          "loading_log",
          format!("Too many images to process ({})", groups.len()),
        )
        .unwrap_or_default();
      break;
    }

    let win_clone = win.clone(); // Clone the Window handle for use in the async block
    let client = client.clone();
    let url_clone = url.clone();

    tasks.push(std::thread::spawn(move || {
      log!("Getting: {}", &url_clone);

      let response = match client.get(request_url).send() {
        Ok(r) => r,
        Err(e) => {
          log!("Request failed: {}", e);
          log!("URL: {}", &url_clone);

          win_clone
            .emit("loading_log", "An image failed to import...".to_string())
            .unwrap();

          return None;
        }
      };

      if !response.status().is_success()
        || response
          .content_length()
          .is_some_and(|length| length > MAX_REMOTE_RESPONSE_BYTES)
      {
        return None;
      }

      let bytes = match response.bytes() {
        Ok(bytes) if bytes.len() <= MAX_REMOTE_RESPONSE_BYTES as usize => bytes,
        _ => return None,
      };
      let b64 = general_purpose::STANDARD.encode(&bytes);

      win_clone
        .emit("loading_log", format!("Processed image import: {}", url_clone))
        .unwrap_or_default();

      if url_clone.is_empty() {
        return None;
      }

      Some((
        url_clone,
        format!("data:image/{filetype};base64,{b64}"),
      ))
    }));
  }

  for task in tasks {
    let result = match task.join() {
      Ok(r) => r,
      Err(e) => {
        log!("Error joining thread: {:?}", e);
        continue;
      }
    };

    if result.is_none() {
      continue;
    }

    let (url, b64) = result.unwrap();

    new_css = new_css.replace(url.as_str(), b64.as_str());
  }

  new_css
}
