use std::{
  collections::HashSet,
  sync::{
    atomic::{AtomicU64, AtomicU8, Ordering},
    Arc,
  },
  thread,
  time::Duration,
};

use serde::Deserialize;
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
use wmi::WMIConnection;

use super::{ProcessEvent, ProcessInfo};
use crate::log;

type EventCallback = Arc<dyn Fn(ProcessEvent) + Send + Sync>;

static WATCHER_GENERATION: AtomicU64 = AtomicU64::new(0);
static FALLBACK_GENERATION: AtomicU64 = AtomicU64::new(0);

#[derive(Deserialize, Debug)]
#[serde(rename = "Win32_ProcessStartTrace")]
struct ProcessStartTrace {
  #[serde(rename = "ProcessID")]
  process_id: u32,
  #[serde(rename = "ProcessName")]
  process_name: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "Win32_ProcessStopTrace")]
struct ProcessStopTrace {
  #[serde(rename = "ProcessID")]
  process_id: u32,
}

#[allow(dead_code)]
pub fn stop_process_watcher() {
  WATCHER_GENERATION.fetch_add(1, Ordering::AcqRel);
}

#[allow(dead_code)]
pub fn start_process_watcher<F>(on_event: F)
where
  F: Fn(ProcessEvent) + Send + Sync + 'static,
{
  let generation = WATCHER_GENERATION.fetch_add(1, Ordering::AcqRel) + 1;
  let on_event: EventCallback = Arc::new(on_event);
  let ready_listeners = Arc::new(AtomicU8::new(0));

  spawn_start_listener(generation, on_event.clone(), ready_listeners.clone());
  spawn_stop_listener(generation, on_event, ready_listeners);
}

fn spawn_start_listener(generation: u64, on_event: EventCallback, ready_listeners: Arc<AtomicU8>) {
  thread::spawn(move || {
    let connection = match WMIConnection::new() {
      Ok(connection) => connection,
      Err(error) => {
        log!("[Windows] Failed to connect to WMI process events: {error}");
        start_polling_fallback(generation, on_event);
        return;
      }
    };
    let events = match connection.notification::<ProcessStartTrace>() {
      Ok(events) => events,
      Err(error) => {
        log!("[Windows] Failed to subscribe to process start events: {error}");
        start_polling_fallback(generation, on_event);
        return;
      }
    };

    mark_listener_ready(generation, &ready_listeners, &on_event);
    log!("[Windows] Listening for process start events through WMI");
    let mut system = System::new();

    for event in events {
      if !is_current_generation(generation) {
        break;
      }

      match event {
        Ok(event) => on_event(ProcessEvent::Started(read_process_info(
          &mut system,
          event.process_id,
          event.process_name,
        ))),
        Err(error) => {
          log!("[Windows] Process start event stream failed: {error}");
          start_polling_fallback(generation, on_event.clone());
          break;
        }
      }
    }
  });
}

fn spawn_stop_listener(generation: u64, on_event: EventCallback, ready_listeners: Arc<AtomicU8>) {
  thread::spawn(move || {
    let connection = match WMIConnection::new() {
      Ok(connection) => connection,
      Err(error) => {
        log!("[Windows] Failed to connect to WMI process events: {error}");
        start_polling_fallback(generation, on_event);
        return;
      }
    };
    let events = match connection.notification::<ProcessStopTrace>() {
      Ok(events) => events,
      Err(error) => {
        log!("[Windows] Failed to subscribe to process stop events: {error}");
        start_polling_fallback(generation, on_event);
        return;
      }
    };

    mark_listener_ready(generation, &ready_listeners, &on_event);
    log!("[Windows] Listening for process stop events through WMI");

    for event in events {
      if !is_current_generation(generation) {
        break;
      }

      match event {
        Ok(event) => on_event(ProcessEvent::Exited {
          pid: u64::from(event.process_id),
        }),
        Err(error) => {
          log!("[Windows] Process stop event stream failed: {error}");
          start_polling_fallback(generation, on_event.clone());
          break;
        }
      }
    }
  });
}

fn mark_listener_ready(generation: u64, ready_listeners: &AtomicU8, on_event: &EventCallback) {
  if ready_listeners.fetch_add(1, Ordering::AcqRel) == 1 && is_current_generation(generation) {
    on_event(ProcessEvent::Resync);
  }
}

fn read_process_info(system: &mut System, pid: u32, fallback_name: String) -> ProcessInfo {
  let pid = Pid::from_u32(pid);
  system.refresh_processes_specifics(
    ProcessesToUpdate::Some(&[pid]),
    true,
    ProcessRefreshKind::nothing()
      .with_exe(UpdateKind::Always)
      .with_cmd(UpdateKind::Always),
  );

  let process = system.process(pid);
  let path = process
    .and_then(|process| process.exe())
    .filter(|path| !path.as_os_str().is_empty())
    .map(|path| path.to_string_lossy().into_owned())
    .unwrap_or(fallback_name);
  let arguments = process.and_then(|process| {
    let arguments = process
      .cmd()
      .iter()
      .skip(1)
      .map(|argument| argument.to_string_lossy())
      .collect::<Vec<_>>()
      .join(" ");
    (!arguments.is_empty()).then_some(arguments)
  });

  ProcessInfo {
    pid: u64::from(pid.as_u32()),
    path,
    arguments,
  }
}

fn start_polling_fallback(generation: u64, on_event: EventCallback) {
  if FALLBACK_GENERATION.swap(generation, Ordering::AcqRel) == generation {
    return;
  }

  thread::spawn(move || {
    log!("[Windows] Falling back to process polling");
    let mut system = System::new();
    refresh_all_processes(&mut system);
    let mut previous_pids: HashSet<Pid> = system.processes().keys().copied().collect();
    on_event(ProcessEvent::Resync);

    while is_current_generation(generation) {
      thread::sleep(Duration::from_secs(5));
      if !is_current_generation(generation) {
        break;
      }

      refresh_all_processes(&mut system);
      let current_pids: HashSet<Pid> = system.processes().keys().copied().collect();

      for pid in current_pids.difference(&previous_pids) {
        let fallback_name = system
          .process(*pid)
          .map(|process| process.name().to_string_lossy().into_owned())
          .unwrap_or_default();
        on_event(ProcessEvent::Started(read_process_info(
          &mut system,
          pid.as_u32(),
          fallback_name,
        )));
      }
      for pid in previous_pids.difference(&current_pids) {
        on_event(ProcessEvent::Exited {
          pid: u64::from(pid.as_u32()),
        });
      }

      previous_pids = current_pids;
    }
  });
}

fn refresh_all_processes(system: &mut System) {
  system.refresh_processes_specifics(
    ProcessesToUpdate::All,
    true,
    ProcessRefreshKind::nothing()
      .with_exe(UpdateKind::OnlyIfNotSet)
      .with_cmd(UpdateKind::OnlyIfNotSet),
  );
}

fn is_current_generation(generation: u64) -> bool {
  WATCHER_GENERATION.load(Ordering::Acquire) == generation
}
