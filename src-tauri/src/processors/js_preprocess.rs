use crate::log;
use crate::util::input_validation::{validate_http_url, validate_payload_size, MAX_JAVASCRIPT_BYTES};

const MAX_IMPORT_URLS: usize = 32;

pub async fn localize_js(url: String) -> String {
  let url = match validate_http_url(&url) {
    Ok(url) => url,
    Err(error) => {
      log!("Skipping invalid JavaScript import URL: {error}");
      return String::new();
    }
  };

  let client = match reqwest::Client::builder()
    .redirect(reqwest::redirect::Policy::none())
    .build()
  {
    Ok(client) => client,
    Err(error) => {
      log!("Failed to create HTTP client: {error}");
      return String::new();
    }
  };

  let response = match client.get(url).send().await {
    Ok(r) => r,
    Err(e) => {
      log!("Request failed: {}", e);
      log!("URL: {}", &url);

      return String::new();
    }
  };

  if !response.status().is_success()
    || response
      .content_length()
      .is_some_and(|length| length > MAX_JAVASCRIPT_BYTES as u64)
  {
    return String::new();
  }

  let script = response.text().await.unwrap_or_default();
  if let Err(error) = validate_payload_size(&script, MAX_JAVASCRIPT_BYTES, "JavaScript import") {
    log!("Skipping oversized JavaScript import: {error}");
    return String::new();
  }

  script
}

#[tauri::command]
pub async fn localize_all_js(urls: Vec<String>) -> Vec<String> {
  let mut localized: Vec<String> = vec![];

  for (index, url) in urls.into_iter().enumerate() {
    if index >= MAX_IMPORT_URLS {
      log!("Skipping JavaScript import beyond the {MAX_IMPORT_URLS} URL limit");
      localized.push(String::new());
      continue;
    }

    localized.push(localize_js(url).await)
  }

  localized
}

pub fn eval_js_imports(window: &tauri::WebviewWindow, scripts: Vec<String>) {
  for script in scripts {
    match window.eval(script.as_str()) {
      Ok(r) => r,
      Err(e) => log(format!("Error evaluating import: {e}")),
    };
  }
}
