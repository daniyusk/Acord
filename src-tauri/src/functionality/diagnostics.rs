use serde::{Deserialize, Serialize};
use std::{
  sync::{Mutex, OnceLock},
  time::{Duration, Instant},
};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System};

use crate::{config::get_config, log};

const MIN_SAMPLE_INTERVAL: Duration = Duration::from_secs(10);
const MAX_PEER_CONNECTIONS: u8 = 64;
const MAX_VIDEO_STREAMS: u8 = 64;
const MAX_BITRATE_BPS: u64 = 10_000_000_000;
const MAX_LATENCY_MS: f64 = 60_000.0;

static LAST_SAMPLE: OnceLock<Mutex<Option<Instant>>> = OnceLock::new();
static PROCESS_SYSTEM: OnceLock<Mutex<System>> = OnceLock::new();

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RtcDiagnosticsSample {
  peer_connections: u8,
  inbound_video_streams: u8,
  outbound_video_streams: u8,
  inbound_bitrate_bps: u64,
  outbound_bitrate_bps: u64,
  packets_lost: u64,
  frames_decoded: u64,
  frames_dropped: u64,
  round_trip_time_ms: Option<f64>,
  jitter_ms: Option<f64>,
}

impl RtcDiagnosticsSample {
  fn validate(&self) -> Result<(), String> {
    if self.peer_connections > MAX_PEER_CONNECTIONS
      || self.inbound_video_streams > MAX_VIDEO_STREAMS
      || self.outbound_video_streams > MAX_VIDEO_STREAMS
    {
      return Err("RTC diagnostic stream count is out of range".to_string());
    }

    if self.inbound_bitrate_bps > MAX_BITRATE_BPS || self.outbound_bitrate_bps > MAX_BITRATE_BPS {
      return Err("RTC diagnostic bitrate is out of range".to_string());
    }

    for (name, value) in [
      ("round-trip time", self.round_trip_time_ms),
      ("jitter", self.jitter_ms),
    ] {
      if let Some(value) = value {
        if !value.is_finite() || !(0.0..=MAX_LATENCY_MS).contains(&value) {
          return Err(format!("RTC diagnostic {name} is out of range"));
        }
      }
    }

    Ok(())
  }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProcessDiagnostics {
  cpu_usage_percent: f32,
  memory_bytes: u64,
  hardware_acceleration_disabled: bool,
  dma_buf_renderer_disabled: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LoggedRtcDiagnosticsSample {
  #[serde(flatten)]
  rtc: RtcDiagnosticsSample,
  process: ProcessDiagnostics,
}

fn process_diagnostics() -> ProcessDiagnostics {
  let pid = Pid::from_u32(std::process::id());
  let system = PROCESS_SYSTEM.get_or_init(|| {
    Mutex::new(System::new_with_specifics(
      RefreshKind::nothing().with_processes(ProcessRefreshKind::everything()),
    ))
  });
  let mut system = system
    .lock()
    .unwrap_or_else(|poisoned| poisoned.into_inner());
  system.refresh_processes_specifics(
    ProcessesToUpdate::Some(&[pid]),
    true,
    ProcessRefreshKind::everything(),
  );
  let process = system.process(pid);
  let config = get_config();

  ProcessDiagnostics {
    cpu_usage_percent: process.map_or(0.0, |process| process.cpu_usage()),
    memory_bytes: process.map_or(0, |process| process.memory()),
    hardware_acceleration_disabled: config.disable_hardware_accel.unwrap_or(false),
    dma_buf_renderer_disabled: std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_some(),
  }
}

#[tauri::command]
pub fn record_rtc_diagnostics(sample: RtcDiagnosticsSample) -> Result<(), String> {
  if !get_config().rtc_diagnostics.unwrap_or(false) {
    return Ok(());
  }

  sample.validate()?;

  let last_sample = LAST_SAMPLE.get_or_init(|| Mutex::new(None));
  let mut last_sample = last_sample
    .lock()
    .map_err(|_| "RTC diagnostics lock is unavailable".to_string())?;
  let now = Instant::now();

  if last_sample.is_some_and(|previous| now.duration_since(previous) < MIN_SAMPLE_INTERVAL) {
    return Ok(());
  }
  *last_sample = Some(now);
  drop(last_sample);

  let logged_sample = LoggedRtcDiagnosticsSample {
    rtc: sample,
    process: process_diagnostics(),
  };
  let serialized = serde_json::to_string(&logged_sample)
    .map_err(|error| format!("Failed to serialize RTC diagnostics: {error}"))?;
  log!("[RTC diagnostics] {serialized}");

  Ok(())
}

#[cfg(test)]
mod tests {
  use super::RtcDiagnosticsSample;

  fn sample() -> RtcDiagnosticsSample {
    RtcDiagnosticsSample {
      peer_connections: 1,
      inbound_video_streams: 1,
      outbound_video_streams: 0,
      inbound_bitrate_bps: 1_000_000,
      outbound_bitrate_bps: 0,
      packets_lost: 0,
      frames_decoded: 60,
      frames_dropped: 0,
      round_trip_time_ms: Some(42.0),
      jitter_ms: Some(3.0),
    }
  }

  #[test]
  fn accepts_bounded_rtc_diagnostics() {
    assert!(sample().validate().is_ok());
  }

  #[test]
  fn rejects_invalid_rtc_diagnostics() {
    let mut invalid = sample();
    invalid.round_trip_time_ms = Some(f64::NAN);
    assert!(invalid.validate().is_err());

    invalid = sample();
    invalid.inbound_bitrate_bps = 10_000_000_001;
    assert!(invalid.validate().is_err());
  }
}
