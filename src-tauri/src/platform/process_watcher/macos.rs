use std::{
  collections::HashSet,
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
  thread,
  time::Duration,
};
use sysinfo::{Pid, ProcessRefreshKind, RefreshKind, System};

use crate::log;

#[allow(dead_code)]
static WATCHER_RUNNING: AtomicBool = AtomicBool::new(false);

#[allow(dead_code)]
pub fn stop_process_watcher() {
  WATCHER_RUNNING.store(false, Ordering::Relaxed);
}

#[allow(dead_code)]
pub fn start_process_watcher<F>(on_change: F)
where
  F: Fn() + Send + Sync + 'static,
{
  if WATCHER_RUNNING.swap(true, Ordering::Relaxed) {
    log!("[macOS] Process watcher is already running");
    return;
  }

  let on_change = Arc::new(on_change);
  let on_change_clone = on_change.clone();

  thread::spawn(move || {
    log!("[macOS] Starting lightweight process watcher...");

    let mut system = System::new_with_specifics(
      RefreshKind::nothing().with_processes(ProcessRefreshKind::nothing()),
    );
    let mut last_pids: HashSet<Pid> = system.processes().keys().copied().collect();

    on_change_clone();

    while WATCHER_RUNNING.load(Ordering::Relaxed) {
      thread::sleep(Duration::from_secs(5));

      if !WATCHER_RUNNING.load(Ordering::Relaxed) {
        break;
      }

      system.refresh_processes_specifics(
        sysinfo::ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing(),
      );

      let current_pids: HashSet<Pid> = system.processes().keys().copied().collect();

      if current_pids != last_pids {
        last_pids = current_pids;
        log!("[macOS] Process change detected!");
        on_change_clone();
      }
    }

    log!("[macOS] Process watcher stopped.");
  });
}
