use std::{
  collections::HashSet,
  sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
  },
  thread,
  time::Duration,
};

use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

use super::{ProcessEvent, ProcessInfo};
use crate::log;

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
  let on_event = Arc::new(on_event);
  thread::spawn(move || {
    log!("[macOS] Starting process polling fallback");
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
        if let Some(process) = process_info(&system, *pid) {
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

fn process_info(system: &System, pid: Pid) -> Option<ProcessInfo> {
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
