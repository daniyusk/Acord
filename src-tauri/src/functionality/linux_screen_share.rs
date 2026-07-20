use serde::Serialize;

#[cfg(target_os = "linux")]
use crate::log;

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
enum UserServiceState {
  Active,
  Inactive,
  Unavailable,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LinuxScreenShareDiagnostics {
  session_type: Option<String>,
  current_desktop: Option<String>,
  runtime_dir_available: bool,
  session_bus_available: bool,
  pipewire_socket_available: bool,
  pipewire_service: UserServiceState,
  portal_service: UserServiceState,
  portal_backend_available: bool,
  recommendations: Vec<&'static str>,
}

struct LinuxScreenShareInput {
  session_type: Option<String>,
  current_desktop: Option<String>,
  runtime_dir_available: bool,
  session_bus_available: bool,
  pipewire_socket_available: bool,
  pipewire_service: UserServiceState,
  portal_service: UserServiceState,
  portal_backend_available: bool,
}

impl LinuxScreenShareDiagnostics {
  fn from_input(input: LinuxScreenShareInput) -> Self {
    let mut recommendations = Vec::new();

    if input.session_type.as_deref() != Some("wayland") {
      recommendations.push(
        "Use a Wayland session when possible; its portal integration is usually more reliable for screen sharing.",
      );
    }

    if !input.runtime_dir_available {
      recommendations
        .push("Launch Acord from a graphical login session so XDG_RUNTIME_DIR is available.");
    }

    if !input.session_bus_available {
      recommendations.push(
        "Ensure the user D-Bus session is running; xdg-desktop-portal is activated through it.",
      );
    }

    if !input.pipewire_socket_available {
      recommendations.push(
        "Check `systemctl --user status pipewire.service`; the PipeWire socket is required for screen sharing.",
      );
    }

    if input.pipewire_service != UserServiceState::Active {
      recommendations.push(
        "Start or enable PipeWire for the current user, then restart Acord before trying to share again.",
      );
    }

    if input.portal_service == UserServiceState::Unavailable {
      recommendations.push(
        "Install and configure xdg-desktop-portal plus a backend matching your desktop environment.",
      );
    }

    if !input.portal_backend_available {
      recommendations.push(
        "Install an xdg-desktop-portal backend matching your desktop environment (for example GTK, GNOME, KDE, wlroots, or Hyprland).",
      );
    }

    Self {
      session_type: input.session_type,
      current_desktop: input.current_desktop,
      runtime_dir_available: input.runtime_dir_available,
      session_bus_available: input.session_bus_available,
      pipewire_socket_available: input.pipewire_socket_available,
      pipewire_service: input.pipewire_service,
      portal_service: input.portal_service,
      portal_backend_available: input.portal_backend_available,
      recommendations,
    }
  }
}

#[cfg(target_os = "linux")]
fn user_service_state(service: &str) -> UserServiceState {
  use std::process::Command;

  match Command::new("systemctl")
    .args(["--user", "is-active", "--quiet", service])
    .status()
  {
    Ok(status) if status.success() => UserServiceState::Active,
    Ok(_) => UserServiceState::Inactive,
    Err(_) => UserServiceState::Unavailable,
  }
}

#[cfg(target_os = "linux")]
fn collect_linux_screen_share_diagnostics() -> LinuxScreenShareDiagnostics {
  use std::{
    env, fs,
    path::{Path, PathBuf},
  };

  let runtime_dir = env::var_os("XDG_RUNTIME_DIR");
  let runtime_dir_available = runtime_dir
    .as_deref()
    .is_some_and(|path| Path::new(path).is_dir());
  let session_bus_available = runtime_dir
    .as_deref()
    .is_some_and(|path| Path::new(path).join("bus").exists());
  let pipewire_socket_available = runtime_dir
    .as_deref()
    .is_some_and(|path| Path::new(path).join("pipewire-0").exists());
  let mut data_dirs: Vec<PathBuf> = env::var_os("XDG_DATA_DIRS")
    .map(|paths| env::split_paths(&paths).collect())
    .unwrap_or_default();
  if data_dirs.is_empty() {
    data_dirs.extend(["/usr/local/share".into(), "/usr/share".into()]);
  }
  if let Some(data_home) = env::var_os("XDG_DATA_HOME")
    .map(PathBuf::from)
    .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
  {
    data_dirs.push(data_home);
  }
  let portal_backend_available = data_dirs.into_iter().any(|path| {
    fs::read_dir(path.join("xdg-desktop-portal").join("portals"))
      .ok()
      .into_iter()
      .flatten()
      .filter_map(Result::ok)
      .any(|entry| {
        entry
          .path()
          .extension()
          .is_some_and(|extension| extension == "portal")
      })
  });

  LinuxScreenShareDiagnostics::from_input(LinuxScreenShareInput {
    session_type: env::var("XDG_SESSION_TYPE").ok(),
    current_desktop: env::var("XDG_CURRENT_DESKTOP").ok(),
    runtime_dir_available,
    session_bus_available,
    pipewire_socket_available,
    pipewire_service: user_service_state("pipewire.service"),
    portal_service: user_service_state("xdg-desktop-portal.service"),
    portal_backend_available,
  })
}

#[cfg(target_os = "linux")]
pub fn log_linux_screen_share_diagnostics() {
  let diagnostics = collect_linux_screen_share_diagnostics();
  let serialized = serde_json::to_string(&diagnostics)
    .unwrap_or_else(|error| format!(r#"{{"serializationError":"{error}"}}"#));

  log!("[Linux screen share] {serialized}");
  for recommendation in diagnostics.recommendations {
    log!("[Linux screen share] {recommendation}");
  }
}

#[cfg(test)]
mod tests {
  use super::{LinuxScreenShareDiagnostics, LinuxScreenShareInput, UserServiceState};

  #[test]
  fn reports_missing_pipewire_and_portal_prerequisites() {
    let diagnostics = LinuxScreenShareDiagnostics::from_input(LinuxScreenShareInput {
      session_type: Some("x11".to_string()),
      current_desktop: None,
      runtime_dir_available: false,
      session_bus_available: false,
      pipewire_socket_available: false,
      pipewire_service: UserServiceState::Inactive,
      portal_service: UserServiceState::Unavailable,
      portal_backend_available: false,
    });

    assert_eq!(diagnostics.recommendations.len(), 7);
    assert!(diagnostics
      .recommendations
      .iter()
      .any(|recommendation| recommendation.contains("PipeWire")));
    assert!(diagnostics
      .recommendations
      .iter()
      .any(|recommendation| recommendation.contains("xdg-desktop-portal")));
  }

  #[test]
  fn accepts_an_healthy_wayland_setup() {
    let diagnostics = LinuxScreenShareDiagnostics::from_input(LinuxScreenShareInput {
      session_type: Some("wayland".to_string()),
      current_desktop: Some("KDE".to_string()),
      runtime_dir_available: true,
      session_bus_available: true,
      pipewire_socket_available: true,
      pipewire_service: UserServiceState::Active,
      portal_service: UserServiceState::Inactive,
      portal_backend_available: true,
    });

    assert!(diagnostics.recommendations.is_empty());
  }
}
