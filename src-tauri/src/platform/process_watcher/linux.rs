use std::{
  collections::HashSet,
  fs,
  path::PathBuf,
  sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
  },
  thread,
  time::Duration,
};

use proc_connector::{ProcConnector, ProcEvent};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

use super::{ProcessEvent, ProcessInfo};
use crate::log;

type EventCallback = Arc<dyn Fn(ProcessEvent) + Send + Sync>;

static WATCHER_GENERATION: AtomicU64 = AtomicU64::new(0);

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

  thread::spawn(move || match ProcConnector::new() {
    Ok(connector) => run_connector(connector, generation, on_event),
    Err(error) => {
      log!("[Linux] Process connector unavailable ({error}); falling back to polling");
      run_polling_fallback(generation, on_event);
    }
  });
}

fn run_connector(connector: ProcConnector, generation: u64, on_event: EventCallback) {
  log!("[Linux] Listening for process exec/exit events through Netlink");
  on_event(ProcessEvent::Resync);
  let mut buffer = vec![0; 4096];

  while is_current_generation(generation) {
    match connector.recv_timeout(&mut buffer, Duration::from_secs(1)) {
      Ok(Some(ProcEvent::Exec { tgid, .. })) => {
        if let Some(process) = read_process_info(tgid) {
          on_event(ProcessEvent::Started(process));
        }
      }
      Ok(Some(ProcEvent::Exit { pid, tgid, .. })) if pid == tgid => {
        on_event(ProcessEvent::Exited {
          pid: u64::from(tgid),
        });
      }
      Ok(Some(_)) | Ok(None) => {}
      Err(error) => {
        log!("[Linux] Process connector failed ({error}); falling back to polling");
        run_polling_fallback(generation, on_event);
        return;
      }
    }
  }
}

fn read_process_info(pid: u32) -> Option<ProcessInfo> {
  let proc_path = PathBuf::from("/proc").join(pid.to_string());
  let command_line = fs::read(proc_path.join("cmdline")).ok()?;
  let mut parts = command_line.split(|byte| *byte == 0);
  let command = parts.next().unwrap_or_default();
  let path = if command.is_empty() {
    fs::read_link(proc_path.join("exe"))
      .ok()?
      .to_string_lossy()
      .into_owned()
  } else {
    String::from_utf8_lossy(command).into_owned()
  };
  let arguments = parts
    .filter(|argument| !argument.is_empty())
    .map(|argument| String::from_utf8_lossy(argument))
    .collect::<Vec<_>>()
    .join(" ");

  Some(ProcessInfo {
    pid: u64::from(pid),
    path,
    arguments: (!arguments.is_empty()).then_some(arguments),
  })
}

fn run_polling_fallback(generation: u64, on_event: EventCallback) {
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
      if let Some(process) = process_info_from_sysinfo(&system, *pid) {
        on_event(ProcessEvent::Started(process));
      }
    }
    for pid in previous_pids.difference(&current_pids) {
      on_event(ProcessEvent::Exited {
        pid: u64::from(pid.as_u32()),
      });
    }

    previous_pids = current_pids;
  }
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

fn process_info_from_sysinfo(system: &System, pid: Pid) -> Option<ProcessInfo> {
  let process = system.process(pid)?;
  let path = process
    .exe()
    .filter(|path| !path.as_os_str().is_empty())
    .map(|path| path.to_string_lossy().into_owned())
    .unwrap_or_else(|| process.name().to_string_lossy().into_owned());
  let arguments = process
    .cmd()
    .iter()
    .skip(1)
    .map(|argument| argument.to_string_lossy())
    .collect::<Vec<_>>()
    .join(" ");

  Some(ProcessInfo {
    pid: u64::from(pid.as_u32()),
    path,
    arguments: (!arguments.is_empty()).then_some(arguments),
  })
}

fn is_current_generation(generation: u64) -> bool {
  WATCHER_GENERATION.load(Ordering::Acquire) == generation
}
