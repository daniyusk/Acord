const DEFAULT_CLIENT_TYPE: &str = "default";
const CANARY_CLIENT_TYPE: &str = "canary";
const PTB_CLIENT_TYPE: &str = "ptb";

fn normalize_client_type(client_type: Option<&str>) -> &'static str {
  match client_type {
    Some(CANARY_CLIENT_TYPE) => CANARY_CLIENT_TYPE,
    Some(PTB_CLIENT_TYPE) => PTB_CLIENT_TYPE,
    _ => DEFAULT_CLIENT_TYPE,
  }
}

pub fn get_client_type() -> &'static str {
  let config = crate::config::get_config();
  normalize_client_type(config.client_type.as_deref())
}

pub fn get_client_url() -> String {
  match get_client_type() {
    DEFAULT_CLIENT_TYPE => "https://discord.com".to_string(),
    client_type => format!("https://{client_type}.discord.com"),
  }
}

pub fn get_client_app_url() -> String {
  let url = get_client_url();
  format!("{url}/app")
}

#[cfg(test)]
mod tests {
  use super::{normalize_client_type, CANARY_CLIENT_TYPE, DEFAULT_CLIENT_TYPE, PTB_CLIENT_TYPE};

  #[test]
  fn accepts_supported_discord_release_channels() {
    assert_eq!(normalize_client_type(Some("default")), DEFAULT_CLIENT_TYPE);
    assert_eq!(normalize_client_type(Some("canary")), CANARY_CLIENT_TYPE);
    assert_eq!(normalize_client_type(Some("ptb")), PTB_CLIENT_TYPE);
  }

  #[test]
  fn falls_back_for_unrecognized_release_channels() {
    assert_eq!(normalize_client_type(None), DEFAULT_CLIENT_TYPE);
    assert_eq!(normalize_client_type(Some("www")), DEFAULT_CLIENT_TYPE);
    assert_eq!(normalize_client_type(Some("attacker")), DEFAULT_CLIENT_TYPE);
  }
}
