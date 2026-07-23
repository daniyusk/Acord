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

/// Stop any running process watcher thread.
#[allow(dead_code)]
pub fn stop_process_watcher() {
  WATCHER_RUNNING.store(false, Ordering::Relaxed);
}

/// Starts an event-driven lightweight process watcher that triggers `on_change`
/// whenever processes start or exit.
/// Performs an initial 1-time scan on boot and diffs PIDs using `ProcessRefreshKind::nothing()`.
#[allow(dead_code)]
pub fn start_process_watcher<F>(on_change: F)
where
  F: Fn() + Send + Sync + 'static,
{
  if WATCHER_RUNNING.swap(true, Ordering::Relaxed) {
    log!("Process watcher is already running");
    return;
  }

  let on_change = Arc::new(on_change);
  let on_change_clone = on_change.clone();

  thread::spawn(move || {
    log!("Starting lightweight process watcher...");

    // 1. Initial 1-time lightweight scan on startup (PIDs + names only, no task/thread/CPU/mem overhead)
    let mut system = System::new_with_specifics(
      RefreshKind::nothing().with_processes(ProcessRefreshKind::nothing()),
    );
    let mut last_pids: HashSet<Pid> = system.processes().keys().copied().collect();

    // Trigger initial check
    on_change_clone();

    // 2. Loop watching for process creation / termination events
    while WATCHER_RUNNING.load(Ordering::Relaxed) {
      thread::sleep(Duration::from_secs(4));

      if !WATCHER_RUNNING.load(Ordering::Relaxed) {
        break;
      }

      system.refresh_processes_specifics(
        sysinfo::ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing(),
      );

      let current_pids: HashSet<Pid> = system.processes().keys().copied().collect();

      // Trigger callback only when a process launches or exits
      if current_pids != last_pids {
        last_pids = current_pids;
        log!("Process change detected by process watcher!");
        on_change_clone();
      }
    }

    log!("Process watcher stopped.");
  });
}
