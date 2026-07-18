#[cfg(not(target_os = "macos"))]
#[cfg(feature = "hotkeys")]
use livesplit_hotkey::Hotkey;
#[cfg(feature = "hotkeys")]
#[cfg(not(target_os = "macos"))]
use livesplit_hotkey::KeyCode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const MAX_KEYBINDS: usize = 64;
const MAX_KEYS_PER_BIND: usize = 8;
const MAX_KEY_FIELD_BYTES: usize = 64;
const MAX_ACTION_BYTES: usize = 128;

#[derive(Serialize, Deserialize, Debug)]
pub struct KeybindChangedEvent {
  pub keys: Vec<KeyStruct>,
  pub key: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KeyStruct {
  pub name: String,
  pub code: String,
}

pub fn validate_keybinds(keybinds: &HashMap<String, Vec<KeyStruct>>) -> Result<(), String> {
  if keybinds.len() > MAX_KEYBINDS {
    return Err(format!("Cannot store more than {MAX_KEYBINDS} keybinds"));
  }

  for (action, keys) in keybinds {
    validate_keybind_action(action)?;
    validate_key_list(keys)?;
  }

  Ok(())
}

pub fn validate_keybind_action(action: &str) -> Result<(), String> {
  if action.is_empty() || action.len() > MAX_ACTION_BYTES || action.chars().any(char::is_control) {
    return Err("Keybind action must be a non-empty, printable string of at most 128 bytes".to_string());
  }

  Ok(())
}

pub fn validate_key_list(keys: &[KeyStruct]) -> Result<(), String> {
  if keys.len() > MAX_KEYS_PER_BIND {
    return Err(format!("A keybind cannot contain more than {MAX_KEYS_PER_BIND} keys"));
  }

  for key in keys {
    for value in [&key.name, &key.code] {
      if value.is_empty()
        || value.len() > MAX_KEY_FIELD_BYTES
        || value.chars().any(char::is_control)
      {
        return Err("Key names and codes must be printable strings of at most 64 bytes".to_string());
      }
    }
  }

  Ok(())
}

#[cfg(test)]
mod tests {
  use std::collections::HashMap;

  use super::{validate_keybinds, KeyStruct};

  #[test]
  fn rejects_unbounded_or_control_character_keybinds() {
    let mut keybinds = HashMap::new();
    keybinds.insert(
      "PUSH_TO_TALK".to_string(),
      vec![KeyStruct {
        name: "Ctrl".to_string(),
        code: "ControlLeft".to_string(),
      }],
    );
    assert!(validate_keybinds(&keybinds).is_ok());

    keybinds.insert("bad\nkey".to_string(), Vec::new());
    assert!(validate_keybinds(&keybinds).is_err());
  }
}

#[cfg(feature = "hotkeys")]
#[cfg(not(target_os = "macos"))]
pub fn js_keycode_to_key(keycode: String) -> Option<KeyCode> {
  match keycode.as_str() {
    // TODO fix for PTT since it uses a slightly different system that doesn't differentiate
    "Control" => Some(KeyCode::ControlLeft),
    "Shift" => Some(KeyCode::ShiftLeft),
    "Alt" => Some(KeyCode::AltLeft),
    "Meta" => Some(KeyCode::MetaLeft),
    _ => {
      use std::str::FromStr;

      KeyCode::from_str(&keycode).ok()
    }
  }
}

#[cfg(feature = "hotkeys")]
#[cfg(not(target_os = "macos"))]
pub fn keystructs_to_hotkey(keys: &[KeyStruct]) -> Option<Hotkey> {
  use livesplit_hotkey::Modifiers;

  let key = keys.last().and_then(|k| js_keycode_to_key(k.code.clone()));
  let mut modifiers = Modifiers::empty();

  if let Some(key_code) = key {
    // Everything from before the last key should be a modifier
    for k in keys.iter().take(keys.len() - 1) {
      if let Some(modifier) = js_keycode_to_key(k.code.clone()) {
        match modifier {
          KeyCode::ControlLeft => modifiers.insert(Modifiers::CONTROL),
          KeyCode::ControlRight => modifiers.insert(Modifiers::CONTROL),
          KeyCode::ShiftLeft => modifiers.insert(Modifiers::SHIFT),
          KeyCode::ShiftRight => modifiers.insert(Modifiers::SHIFT),
          KeyCode::AltLeft => modifiers.insert(Modifiers::ALT),
          KeyCode::AltRight => modifiers.insert(Modifiers::ALT),
          KeyCode::MetaLeft => modifiers.insert(Modifiers::META),
          KeyCode::MetaRight => modifiers.insert(Modifiers::META),
          _ => continue,
        }
      }
    }

    return Some(Hotkey {
      key_code,
      modifiers,
    });
  }

  None
}
