use super::ProcessEvent;
use crate::log;

#[allow(dead_code)]
pub fn stop_process_watcher() {}

#[allow(dead_code)]
pub fn start_process_watcher<F>(_on_event: F)
where
  F: Fn(ProcessEvent) + Send + Sync + 'static,
{
  log!("Process watcher is not supported on this platform.");
}
