use crate::log;

#[allow(dead_code)]
pub fn stop_process_watcher() {}

#[allow(dead_code)]
pub fn start_process_watcher<F>(_on_change: F)
where
  F: Fn() + Send + Sync + 'static,
{
  log!("Process watcher is not supported on this platform.");
}
