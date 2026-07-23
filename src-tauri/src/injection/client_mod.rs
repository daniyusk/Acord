use phf::phf_map;

use crate::{config::get_config, log};

pub struct ClientMod {
  script: &'static str,
  styles: &'static str,
}

pub static CLIENT_MODS: phf::Map<&'static str, ClientMod> = phf_map! {
  "Shelter" => ClientMod {
    script: "https://raw.githubusercontent.com/uwu/shelter-builds/main/shelter.js",
    styles: "",
  },
  "Vencord" => ClientMod {
    script: "https://github.com/Vendicated/Vencord/releases/download/devbuild/browser.js",
    styles: "https://github.com/Vendicated/Vencord/releases/download/devbuild/browser.css",
  },
  "Equicord" => ClientMod {
    script: "https://github.com/Equicord/Equicord/releases/download/latest/browser.js",
    styles: "https://github.com/Equicord/Equicord/releases/download/latest/browser.css",
  },
};

fn enabled_client_mods() -> Vec<String> {
  filter_enabled_client_mods(get_config().client_mods.clone().unwrap_or_default())
}

fn filter_enabled_client_mods(configured_mods: Vec<String>) -> Vec<String> {
  let mut enabled_mods = Vec::new();

  for mod_name in configured_mods {
    if CLIENT_MODS.contains_key(mod_name.as_str()) && !enabled_mods.contains(&mod_name) {
      enabled_mods.push(mod_name);
    }
  }

  enabled_mods
}

#[tauri::command]
pub fn available_mods() -> Vec<String> {
  CLIENT_MODS.keys().map(|s| s.to_string()).collect()
}

pub fn load_mods_js() -> String {
  let enabled_mods = enabled_client_mods();

  let mut exec = String::new();
  let mut tasks = Vec::new();

  for mod_name in enabled_mods {
    let script_url = CLIENT_MODS
      .get(mod_name.as_str())
      .unwrap_or(
        // Prevent panics
        &ClientMod {
          script: "",
          styles: "",
        },
      )
      .script;

    tasks.push(std::thread::spawn(move || {
      let response = match tauri::async_runtime::block_on(async { reqwest::get(script_url).await }) {
        Ok(r) => r,
        Err(e) => {
          log!("Failed to load client mod JS for {}: {:?}", mod_name, e);

          return String::new();
        }
      };

      let status = response.status();

      if status != 200 {
        log!(
          "Failed to load client mod JS for {}: Status {:?}",
          mod_name,
          status
        );

        return String::new();
      }

      tauri::async_runtime::block_on(async { response.text().await }).unwrap_or_default()
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

    log!("Joining (load_mods_js)...");

    if result.is_empty() {
      continue;
    }

    exec = format!("{exec};{result}");
  }

  exec
}

#[cfg(test)]
mod tests {
  use super::filter_enabled_client_mods;

  #[test]
  fn ignores_unknown_and_duplicate_client_mods() {
    let enabled = filter_enabled_client_mods(vec![
      "Unknown".to_string(),
      "Shelter".to_string(),
      "Shelter".to_string(),
      "Vencord".to_string(),
    ]);

    assert_eq!(enabled, vec!["Shelter".to_string(), "Vencord".to_string()]);
  }
}

#[tauri::command]
pub fn load_mods_css() -> String {
  let enabled_mods = enabled_client_mods();
  let mut exec = String::new();

  let mut tasks = Vec::new();

  for mod_name in enabled_mods {
    let styles_url = CLIENT_MODS
      .get(mod_name.as_str())
      .unwrap_or(
        // Prevent panics
        &ClientMod {
          script: "",
          styles: "",
        },
      )
      .styles;

    if styles_url.is_empty() {
      continue;
    }

    tasks.push(std::thread::spawn(move || {
      let response = match tauri::async_runtime::block_on(async { reqwest::get(styles_url).await }) {
        Ok(r) => r,
        Err(e) => {
          log!("Failed to load client mod CSS for {}: {:?}", mod_name, e);
          return String::new();
        }
      };

      let status = response.status();

      if status != 200 {
        log!(
          "Failed to load client mod CSS for {}: Status {:?}",
          mod_name,
          status
        );
        return String::new();
      }

      tauri::async_runtime::block_on(async { response.text().await }).unwrap_or_default()
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

    log!("Joining (load_mods_css)...");

    if result.is_empty() {
      continue;
    }

    exec = format!("{exec};{result}");
  }

  exec
}
