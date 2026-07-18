#[cfg(not(target_os = "macos"))]
#[cfg(feature = "hotkeys")]
use livesplit_hotkey::Hotkey;
#[cfg(feature = "hotkeys")]
#[cfg(not(target_os = "macos"))]
use livesplit_hotkey::KeyCode;
use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::fmt;

const MAX_KEYBINDS: usize = 64;
const MAX_KEYS_PER_BIND: usize = 8;
const MAX_KEY_FIELD_BYTES: usize = 64;
const MAX_ACTION_BYTES: usize = 128;
pub const PUSH_TO_TALK_ACTION: &str = "PUSH_TO_TALK";
const PUSH_ACTION_PREFIX: &str = "PUSH";

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum KeybindAction {
  PushToTalk,
  Push(String),
  Custom(String),
}

impl KeybindAction {
  pub fn parse(value: String) -> Result<Self, String> {
    validate_keybind_action(&value)?;

    if value == PUSH_TO_TALK_ACTION {
      Ok(Self::PushToTalk)
    } else if value.starts_with(PUSH_ACTION_PREFIX) {
      Ok(Self::Push(value))
    } else {
      Ok(Self::Custom(value))
    }
  }

  pub fn as_str(&self) -> &str {
    match self {
      Self::PushToTalk => PUSH_TO_TALK_ACTION,
      Self::Push(value) | Self::Custom(value) => value,
    }
  }

  #[cfg(feature = "hotkeys")]
  pub fn is_push_action(&self) -> bool {
    matches!(self, Self::PushToTalk | Self::Push(_))
  }

  #[cfg(feature = "hotkeys")]
  pub fn is_push_to_talk(&self) -> bool {
    matches!(self, Self::PushToTalk)
  }
}

impl fmt::Display for KeybindAction {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str(self.as_str())
  }
}

impl Serialize for KeybindAction {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    serializer.serialize_str(self.as_str())
  }
}

impl<'de> Deserialize<'de> for KeybindAction {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    Self::parse(String::deserialize(deserializer)?).map_err(D::Error::custom)
  }
}

#[derive(Serialize, Deserialize, Debug)]
#[cfg(all(feature = "hotkeys", not(target_os = "macos")))]
#[cfg(feature = "hotkeys")]
#[cfg(not(target_os = "macos"))]
pub struct KeybindChangedEvent {
  pub keys: Vec<KeyStruct>,
  pub key: KeybindAction,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KeyStruct {
  pub name: String,
  pub code: String,
}

pub fn validate_keybinds(keybinds: &HashMap<KeybindAction, Vec<KeyStruct>>) -> Result<(), String> {
  if keybinds.len() > MAX_KEYBINDS {
    return Err(format!("Cannot store more than {MAX_KEYBINDS} keybinds"));
  }

  for keys in keybinds.values() {
    validate_key_list(keys)?;
  }

  Ok(())
}

pub fn validate_keybind_action(action: &str) -> Result<(), String> {
  if action.is_empty() || action.len() > MAX_ACTION_BYTES || action.chars().any(char::is_control) {
    return Err(
      "Keybind action must be a non-empty, printable string of at most 128 bytes".to_string(),
    );
  }

  Ok(())
}

pub fn validate_key_list(keys: &[KeyStruct]) -> Result<(), String> {
  if keys.len() > MAX_KEYS_PER_BIND {
    return Err(format!(
      "A keybind cannot contain more than {MAX_KEYS_PER_BIND} keys"
    ));
  }

  for key in keys {
    for value in [&key.name, &key.code] {
      if value.is_empty()
        || value.len() > MAX_KEY_FIELD_BYTES
        || value.chars().any(char::is_control)
      {
        return Err(
          "Key names and codes must be printable strings of at most 64 bytes".to_string(),
        );
      }
    }
  }

  Ok(())
}

#[cfg(test)]
mod tests {
  use std::collections::HashMap;

  use super::{validate_keybinds, KeyStruct, KeybindAction, PUSH_TO_TALK_ACTION};

  #[test]
  fn rejects_unbounded_or_control_character_keybinds() {
    let mut keybinds = HashMap::new();
    keybinds.insert(
      KeybindAction::PushToTalk,
      vec![KeyStruct {
        name: "Ctrl".to_string(),
        code: "ControlLeft".to_string(),
      }],
    );
    assert!(validate_keybinds(&keybinds).is_ok());

    assert!(KeybindAction::parse("bad\nkey".to_string()).is_err());
  }

  #[test]
  fn rejects_keybinds_with_too_many_or_oversized_keys() {
    let key = KeyStruct {
      name: "Ctrl".to_string(),
      code: "ControlLeft".to_string(),
    };
    let mut keybinds = HashMap::new();
    keybinds.insert(KeybindAction::PushToTalk, vec![key.clone(); 9]);
    assert!(validate_keybinds(&keybinds).is_err());

    keybinds.insert(
      KeybindAction::PushToTalk,
      vec![KeyStruct {
        name: "Ctrl".to_string(),
        code: "a".repeat(65),
      }],
    );
    assert!(validate_keybinds(&keybinds).is_err());
  }

  #[test]
  fn serializes_known_and_custom_actions_as_legacy_string_keys() {
    let custom = KeybindAction::parse("TOGGLE_MUTE".to_string()).unwrap();
    let custom_push = KeybindAction::parse("PUSH_CUSTOM".to_string()).unwrap();
    let mut keybinds = HashMap::new();
    keybinds.insert(KeybindAction::PushToTalk, Vec::<KeyStruct>::new());

    assert_eq!(KeybindAction::PushToTalk.as_str(), PUSH_TO_TALK_ACTION);
    assert!(matches!(
      KeybindAction::PushToTalk,
      KeybindAction::PushToTalk
    ));
    assert!(matches!(custom, KeybindAction::Custom(_)));
    assert!(matches!(custom_push, KeybindAction::Push(_)));
    assert_eq!(
      serde_json::to_string(&KeybindAction::PushToTalk).unwrap(),
      "\"PUSH_TO_TALK\""
    );
    assert_eq!(serde_json::to_string(&custom).unwrap(), "\"TOGGLE_MUTE\"");
    assert_eq!(
      serde_json::to_string(&keybinds).unwrap(),
      "{\"PUSH_TO_TALK\":[]}"
    );
    assert!(
      serde_json::from_str::<HashMap<KeybindAction, Vec<KeyStruct>>>("{\"PUSH_TO_TALK\":[]}")
        .unwrap()
        .contains_key(&KeybindAction::PushToTalk)
    );
    assert_eq!(
      serde_json::from_str::<KeybindAction>("\"PUSH_TO_TALK\"").unwrap(),
      KeybindAction::PushToTalk
    );
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
