use std::{
  net::IpAddr,
  path::{Component, Path},
};

use reqwest::Url;

pub const MAX_CSS_BYTES: usize = 1024 * 1024;
pub const MAX_JAVASCRIPT_BYTES: usize = 1024 * 1024;
pub const MAX_REMOTE_RESPONSE_BYTES: u64 = 5 * 1024 * 1024;
const MAX_FILE_NAME_BYTES: usize = 200;
const MAX_URL_BYTES: usize = 2048;

pub fn validate_http_url(value: &str) -> Result<Url, String> {
  if value.is_empty() || value.len() > MAX_URL_BYTES {
    return Err("URL must be between 1 and 2048 bytes".to_string());
  }

  let url = Url::parse(value).map_err(|error| format!("Invalid URL: {error}"))?;

  if !matches!(url.scheme(), "http" | "https") {
    return Err("Only HTTP and HTTPS URLs are allowed".to_string());
  }

  if !url.username().is_empty() || url.password().is_some() {
    return Err("URLs with credentials are not allowed".to_string());
  }

  let host = url.host_str().ok_or("URL must include a host")?;
  if host.eq_ignore_ascii_case("localhost") || host.to_ascii_lowercase().ends_with(".localhost") {
    return Err("Localhost URLs are not allowed".to_string());
  }

  // `Url::host_str` retains brackets around IPv6 literals, while `IpAddr`
  // expects the bare address.
  let ip_host = host
    .strip_prefix('[')
    .and_then(|value| value.strip_suffix(']'))
    .unwrap_or(host);
  if let Ok(ip) = ip_host.parse::<IpAddr>() {
    validate_public_ip(ip)?;
  }

  Ok(url)
}

pub fn validate_file_name(value: &str) -> Result<(), String> {
  if value.is_empty() {
    return Err("File name cannot be empty".to_string());
  }

  if value.len() > MAX_FILE_NAME_BYTES {
    return Err("File name cannot exceed 200 bytes".to_string());
  }

  if value.chars().any(|character| {
    character.is_control()
      || matches!(character, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
  }) {
    return Err("File name contains an invalid path character".to_string());
  }

  let mut components = Path::new(value).components();
  if !matches!(components.next(), Some(Component::Normal(_))) || components.next().is_some() {
    return Err("File name must be a single path component".to_string());
  }

  Ok(())
}

pub fn normalize_css_file_name(value: &str) -> Result<String, String> {
  let mut name = value.to_string();

  if name.to_ascii_lowercase().ends_with(".css") {
    name.truncate(name.len() - ".css".len());
  }

  if name.is_empty() {
    return Err("Theme name cannot be empty".to_string());
  }

  let filename = format!("{name}.css");
  validate_file_name(&filename)?;

  Ok(filename)
}

pub fn validate_payload_size(value: &str, max_bytes: usize, label: &str) -> Result<(), String> {
  if value.len() > max_bytes {
    return Err(format!("{label} cannot exceed {max_bytes} bytes"));
  }

  Ok(())
}

pub fn is_discord_snowflake(value: &str) -> bool {
  !value.is_empty() && value.len() <= 20 && value.chars().all(|character| character.is_ascii_digit())
}

fn validate_public_ip(ip: IpAddr) -> Result<(), String> {
  if let IpAddr::V6(ip) = ip {
    if let Some(mapped_ipv4) = ip.to_ipv4_mapped() {
      return validate_public_ip(IpAddr::V4(mapped_ipv4));
    }
  }

  let blocked = match ip {
    IpAddr::V4(ip) => {
      ip.is_private()
        || ip.is_loopback()
        || ip.is_link_local()
        || ip.is_broadcast()
        || ip.is_unspecified()
        || ip.is_multicast()
        || matches!(ip.octets(), [100, 64..=127, _, _] | [198, 18..=19, _, _])
    }
    IpAddr::V6(ip) => {
      ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_unique_local()
        || ip.is_unicast_link_local()
        || ip.is_multicast()
    }
  };

  if blocked {
    Err("Local or reserved IP addresses are not allowed".to_string())
  } else {
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::{
    is_discord_snowflake, normalize_css_file_name, validate_file_name, validate_http_url,
    validate_payload_size,
  };

  #[test]
  fn accepts_public_http_urls() {
    assert!(validate_http_url("https://example.com/theme.css").is_ok());
    assert!(validate_http_url("http://8.8.8.8/image.png").is_ok());
  }

  #[test]
  fn rejects_unsafe_urls() {
    let oversized_url = format!("https://example.com/{}", "a".repeat(2048));
    for url in [
      "file:///etc/passwd",
      "https://localhost/icon.png",
      "https://LOCALHOST/icon.png",
      "http://127.0.0.1:8080/",
      "http://192.168.1.1/",
      "http://[::1]/",
      "http://[::ffff:127.0.0.1]/",
      "https://user:password@example.com/",
      oversized_url.as_str(),
    ] {
      assert!(validate_http_url(url).is_err(), "{url} should be rejected");
    }
  }

  #[test]
  fn normalizes_and_validates_css_file_names() {
    assert_eq!(normalize_css_file_name("Midnight.CSS").unwrap(), "Midnight.css");
    assert_eq!(normalize_css_file_name("midnight").unwrap(), "midnight.css");
    assert!(normalize_css_file_name("").is_err());
    assert!(validate_file_name("theme.css").is_ok());

    let oversized = "a".repeat(201);
    for name in [
      "../theme.css",
      "nested/theme.css",
      "C:\\theme.css",
      "theme\0.css",
      oversized.as_str(),
    ] {
      assert!(validate_file_name(name).is_err(), "{name:?} should be rejected");
    }
  }

  #[test]
  fn rejects_oversized_payloads_and_invalid_snowflakes() {
    assert!(validate_payload_size("safe", 4, "Payload").is_ok());
    assert!(validate_payload_size("oversized", 4, "Payload").is_err());

    assert!(is_discord_snowflake("12345678901234567890"));
    for value in ["", "123abc", "123\n456", "123456789012345678901"] {
      assert!(!is_discord_snowflake(value), "{value:?} should be rejected");
    }
  }
}
