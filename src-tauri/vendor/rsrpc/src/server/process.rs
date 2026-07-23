use aho_corasick::{AhoCorasick, PatternID};
use std::collections::HashSet;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use std::vec;

#[cfg(not(target_os = "linux"))]
use sysinfo::System;

use crate::log;
use crate::ProcessCallback;

use super::super::DetectableActivity;

#[derive(Default, Clone)]
pub struct ProcessScanState {
  pub obs_open: bool,
}

#[derive(Default)]
pub struct ProcessEventListeners {
  pub on_process_scan_complete: Option<Arc<Mutex<ProcessCallback>>>,
}

#[derive(Clone)]
pub struct Exec {
  pub pid: u64,
  pub path: String,
  pub arguments: Option<String>,
}

#[derive(Clone)]
pub struct ProcessDetectedEvent {
  pub activity: Arc<DetectableActivity>,
}

#[derive(Clone)]
pub struct ProcessServer {
  detected_list: Arc<Mutex<Vec<Arc<DetectableActivity>>>>,
  custom_detectables: Arc<Mutex<Vec<Arc<DetectableActivity>>>>,
  scanning: Arc<AtomicBool>,
  obs_pids: Arc<Mutex<HashSet<u64>>>,

  detectable_indexes: Arc<Mutex<Vec<[usize; 2]>>>,
  detectable_ac: Arc<Mutex<AhoCorasick>>,

  custom_detectable_indexes: Arc<Mutex<Vec<[usize; 2]>>>,
  custom_detectable_ac: Arc<Mutex<Option<AhoCorasick>>>,

  pub detectable_list: Vec<Arc<DetectableActivity>>,
  pub event_sender: mpsc::Sender<ProcessDetectedEvent>,

  event_listeners: Arc<Mutex<ProcessEventListeners>>,

  #[cfg(not(target_os = "linux"))]
  sysinfo: Arc<Mutex<System>>,
}

unsafe impl Sync for ProcessServer {}

impl ProcessServer {
  pub fn new(
    detectable: Vec<Arc<DetectableActivity>>,
    event_sender: mpsc::Sender<ProcessDetectedEvent>,
    event_listeners: ProcessEventListeners,
  ) -> Self {
    log!("[Process Scanner] Building Aho-Corasick patterns for main detectable activities...");
    let (ac, idx) = build_ac_patterns(&detectable);
    log!("[Process Scanner] Done!");

    ProcessServer {
      scanning: Arc::new(AtomicBool::new(false)),
      detected_list: Arc::new(Mutex::new(vec![])),
      custom_detectables: Arc::new(Mutex::new(vec![])),
      obs_pids: Arc::new(Mutex::new(HashSet::new())),
      detectable_list: detectable,
      event_sender,

      // Aho-Corasick matching with detectables mapping
      detectable_indexes: Arc::new(Mutex::new(idx)),
      detectable_ac: Arc::new(Mutex::new(ac)),
      custom_detectable_indexes: Arc::new(Mutex::new(vec![])),
      custom_detectable_ac: Arc::new(Mutex::new(None)),

      // Event listeners
      event_listeners: Arc::new(Mutex::new(event_listeners)),

      // sysinfo System
      #[cfg(not(target_os = "linux"))]
      sysinfo: Arc::new(Mutex::new(System::new())),
    }
  }

  fn update_custom_detectables(&self) {
    log!("[Process Scanner] Updating Aho-Corasick patterns for custom detectable activities...");
    let (ac, idx) = build_ac_patterns(&self.custom_detectables.lock().unwrap());
    if !idx.is_empty() {
      *self.custom_detectable_ac.lock().unwrap() = Some(ac);
    } else {
      *self.custom_detectable_ac.lock().unwrap() = None;
    }
    *self.custom_detectable_indexes.lock().unwrap() = idx;
    log!("[Process Scanner] Done!");
  }

  pub fn append_detectables(&mut self, detectable: Vec<DetectableActivity>) {
    // Append to detectable chunks, since that's what is actually scanned
    self
      .custom_detectables
      .lock()
      .unwrap()
      .extend(detectable.into_iter().map(Arc::new));
    self.update_custom_detectables();
  }

  pub fn remove_detectable_by_name(&mut self, name: String) {
    self
      .custom_detectables
      .lock()
      .unwrap()
      .retain(|x| x.name != name);
    self.update_custom_detectables();

    let previous = self.active_process();
    self
      .detected_list
      .lock()
      .unwrap()
      .retain(|activity| activity.name != name);
    self.publish_if_active_changed(previous);
  }

  pub fn start(&self) {
    let wait_time = Duration::from_secs(10);
    let clone = self.clone();

    self.update_custom_detectables();

    std::thread::spawn(move || {
      // Run the process scan repeatedly (every 3 seconds)
      loop {
        if let Err(err) = clone.scan_and_publish() {
          log!("[Process Scanner] Error while scanning processes: {}", err);
        }

        std::thread::sleep(wait_time);
      }
    });
  }

  pub fn process_started(
    &self,
    pid: u64,
    path: String,
    arguments: Option<String>,
  ) -> Result<(), Box<dyn std::error::Error>> {
    let process = Exec {
      pid,
      path,
      arguments,
    };
    let is_obs = is_obs_process(&process.path);
    let obs_changed = {
      let mut obs_pids = self.obs_pids.lock().unwrap();
      if is_obs {
        obs_pids.insert(pid)
      } else {
        obs_pids.remove(&pid)
      }
    };

    if obs_changed {
      self.notify_scan_state();
    }

    let detected = self.match_process(&process);
    let previous = self.active_process();
    let mut detected_list = self.detected_list.lock().unwrap();

    if let Some(existing) = detected_list
      .iter()
      .position(|activity| activity.pid == Some(pid))
    {
      if detected
        .as_ref()
        .is_some_and(|activity| activity.id == detected_list[existing].id)
      {
        return Ok(());
      }
      detected_list.remove(existing);
    }

    if let Some(activity) = detected {
      detected_list.push(activity);
    }
    drop(detected_list);

    self.publish_if_active_changed(previous);
    Ok(())
  }

  pub fn process_exited(&self, pid: u64) {
    let obs_changed = self.obs_pids.lock().unwrap().remove(&pid);
    if obs_changed {
      self.notify_scan_state();
    }

    let previous = self.active_process();
    self
      .detected_list
      .lock()
      .unwrap()
      .retain(|activity| activity.pid != Some(pid));
    self.publish_if_active_changed(previous);
  }

  pub fn scan_and_publish(&self) -> Result<(), Box<dyn std::error::Error>> {
    let previous = self.active_process();
    let detected = self.scan_for_processes()?;
    *self.detected_list.lock().unwrap() = detected;
    self.publish_if_active_changed(previous);
    Ok(())
  }

  fn active_process(&self) -> Option<(String, u64)> {
    self
      .detected_list
      .lock()
      .unwrap()
      .first()
      .map(|activity| (activity.id.clone(), activity.pid.unwrap_or_default()))
  }

  fn publish_if_active_changed(&self, previous: Option<(String, u64)>) {
    if self.active_process() == previous {
      return;
    }

    let activity = self
      .detected_list
      .lock()
      .unwrap()
      .first()
      .cloned()
      .unwrap_or_else(empty_activity);

    if self
      .event_sender
      .send(ProcessDetectedEvent { activity })
      .is_err()
    {
      log!("[Process Scanner] Process event receiver disconnected");
    }
  }

  fn notify_scan_state(&self) {
    if let Some(callback) = self
      .event_listeners
      .lock()
      .unwrap()
      .on_process_scan_complete
      .as_ref()
    {
      callback.lock().unwrap()(ProcessScanState {
        obs_open: !self.obs_pids.lock().unwrap().is_empty(),
      });
    }
  }

  fn match_process(&self, process: &Exec) -> Option<Arc<DetectableActivity>> {
    let mut process_path = process.path.to_ascii_lowercase();

    if process_path.contains('\\') {
      process_path = process_path.replace('\\', "/");
    }

    if !process_path.starts_with('/') {
      process_path.insert(0, '/');
    }

    let reversed_path: String = process_path.chars().rev().collect();
    let ac = self.detectable_ac.lock().unwrap();
    let custom_ac = self.custom_detectable_ac.lock().unwrap();

    let (obj, exe_index) = if let Some(mat) = ac.find(&reversed_path) {
      let pattern_id: PatternID = mat.pattern();
      let exe_index = self.detectable_indexes.lock().unwrap()[pattern_id.as_usize()];
      (self.detectable_list[exe_index[0]].clone(), exe_index[1])
    } else if let Some(custom_ac) = custom_ac.as_ref() {
      if let Some(mat) = custom_ac.find(&reversed_path) {
        let pattern_id: PatternID = mat.pattern();
        let exe_index = self.custom_detectable_indexes.lock().unwrap()[pattern_id.as_usize()];
        (
          self.custom_detectables.lock().unwrap()[exe_index[0]].clone(),
          exe_index[1],
        )
      } else {
        return None;
      }
    } else {
      return None;
    };

    let executable = &obj.executables.as_ref().unwrap()[exe_index];
    if let Some(exec_args) = &executable.arguments {
      if executable.name.starts_with('>')
        && !process
          .arguments
          .as_ref()
          .is_some_and(|args| args.contains(exec_args))
      {
        return None;
      }
    }

    let mut new_activity = (*obj).clone();
    new_activity.pid = Some(process.pid);
    new_activity.timestamp = Some(unix_timestamp_millis());
    Some(Arc::new(new_activity))
  }

  #[cfg(not(target_os = "linux"))]
  pub fn process_list(&self) -> Result<Vec<Exec>, Box<dyn std::error::Error>> {
    use std::path::Path;
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, UpdateKind};

    let mut processes = Vec::new();
    let mut sys = self.sysinfo.lock().unwrap();
    sys.refresh_processes_specifics(
      ProcessesToUpdate::All,
      true,
      ProcessRefreshKind::nothing()
        .with_exe(UpdateKind::OnlyIfNotSet)
        .with_cmd(UpdateKind::OnlyIfNotSet),
    );

    for proc in sys.processes() {
      let mut cmd = proc.1.cmd().iter();
      processes.push(Exec {
        pid: proc.0.to_string().parse::<u64>()?,
        path: proc.1.exe().unwrap_or(Path::new("")).display().to_string(),
        arguments: cmd.next().map(|_| {
          cmd
            .map(|x| x.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ")
        }),
      });
    }

    Ok(processes)
  }

  #[cfg(target_os = "linux")]
  pub fn process_list() -> Result<Vec<Exec>, Box<dyn std::error::Error>> {
    use std::fs;

    let proc_list = fs::read_dir("/proc")?.filter(|e| {
      if let Ok(entry) = e {
        // Only if we can parse this as a number
        return entry.file_name().to_str().unwrap().parse::<u64>().is_ok();
      }

      false
    });
    let mut processes = Vec::new();

    for entry in proc_list {
      let entry = entry?;
      let path = entry.path();

      if let Ok(cmdline) = fs::read_to_string(path.join("cmdline")) {
        if !cmdline.is_empty() {
          let mut cmd_iter = cmdline.split('\0');
          let (cmd_path, cmd_args) = (
            cmd_iter.next().unwrap_or("").to_string(),
            cmd_iter.collect::<Vec<_>>().join(" "),
          );
          processes.push(Exec {
            pid: path
              .file_name()
              .ok_or("Invalid path")?
              .to_str()
              .ok_or("Invalid path")?
              .parse::<u64>()?,
            path: cmd_path,
            arguments: if cmd_args.is_empty() {
              None
            } else {
              Some(cmd_args)
            },
          });
        }
      }
    }

    Ok(processes)
  }

  pub fn scan_for_processes(
    &self,
  ) -> Result<Vec<Arc<DetectableActivity>>, Box<dyn std::error::Error>> {
    log!("[Process Scanner] Process scan triggered");

    if self
      .scanning
      .swap(true, std::sync::atomic::Ordering::AcqRel)
    {
      log!("[Process Scanner] Scanning already in progress");
      return Err("Scanning already in progress".into());
    }
    let _scan_guard = ScanGuard(&self.scanning);

    #[cfg(not(target_os = "linux"))]
    let processes = self.process_list()?;
    #[cfg(target_os = "linux")]
    let processes = ProcessServer::process_list()?;

    let mut obs_open = false;

    let ac = self.detectable_ac.lock().unwrap();
    let custom_ac = self.custom_detectable_ac.lock().unwrap();

    let mut reversed_path = String::with_capacity(256);

    let mut detected_list: Vec<Arc<DetectableActivity>> = processes
      .iter()
      .filter_map(|process| {
        // Process path (but consistent slashes, so we can compare properly)
        let mut process_path = process.path.to_ascii_lowercase();

        if process_path.contains('\\') {
          process_path = process_path.replace('\\', "/");
        }

        if !process_path.starts_with('/') {
          process_path.insert(0, '/');
        }

        if !obs_open && (process_path.contains("obs64") || process_path.contains("streamlabs")) {
          obs_open = true;
        }

        // Aho-Corasick matching
        reversed_path.clear();
        reversed_path.extend(process_path.chars().rev());

        let (obj, exe_index) = if let Some(mat) = ac.find(&reversed_path) {
          let pattern_id: PatternID = mat.pattern();
          let exe_index = self.detectable_indexes.lock().unwrap()[pattern_id.as_usize()];
          (&self.detectable_list[exe_index[0]], exe_index[1])
        } else if let Some(custom_ac) = custom_ac.as_ref() {
          if let Some(mat) = custom_ac.find(&reversed_path) {
            let pattern_id: PatternID = mat.pattern();
            let exe_index = self.custom_detectable_indexes.lock().unwrap()[pattern_id.as_usize()];
            (
              &self.custom_detectables.lock().unwrap()[exe_index[0]],
              exe_index[1],
            )
          } else {
            return None;
          }
        } else {
          return None;
        };

        // Argument checks
        let executable = &obj.executables.as_ref().unwrap()[exe_index];

        if let Some(exec_args) = &executable.arguments {
          // Only require argument checks if executable starts with '>'
          // like Minecraft: { arguments: "net.minecraft.client.main.Main", is_launcher: false, name: ">java", â€¦ }
          // Other games might provide arguments but not necessary be checked
          // like Left 4 Dead 2: { arguments: "-game left4dead2", is_launcher: false, name: "left 4 dead 2/left4dead2.exe", â€¦ }
          if executable.name.starts_with(">")
            && !process
              .arguments
              .as_ref()
              .is_some_and(|args| args.contains(exec_args))
          {
            return None;
          }
        }

        let mut new_activity = (**obj).clone();
        new_activity.pid = Some(process.pid);
        new_activity.timestamp = Some(format!(
          "{:?}",
          std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
        ));
        Some(Arc::new(new_activity))
      })
      .collect();

    *self.obs_pids.lock().unwrap() = processes
      .iter()
      .filter(|process| is_obs_process(&process.path))
      .map(|process| process.pid)
      .collect();

    if let Some(callback) = self
      .event_listeners
      .lock()
      .unwrap()
      .on_process_scan_complete
      .as_ref()
    {
      callback.lock().unwrap()(ProcessScanState { obs_open });
    }

    detected_list.shrink_to_fit();

    log!("[Process Scanner] Process scan complete");

    Ok(detected_list)
  }
}

struct ScanGuard<'a>(&'a AtomicBool);

impl Drop for ScanGuard<'_> {
  fn drop(&mut self) {
    self.0.store(false, std::sync::atomic::Ordering::Release);
  }
}

fn is_obs_process(path: &str) -> bool {
  let path = path.to_ascii_lowercase();
  path.contains("obs64") || path.contains("streamlabs")
}

fn unix_timestamp_millis() -> String {
  std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap_or_default()
    .as_millis()
    .to_string()
}

fn empty_activity() -> Arc<DetectableActivity> {
  Arc::new(DetectableActivity {
    bot_public: None,
    bot_require_code_grant: None,
    cover_image: None,
    description: None,
    developers: None,
    executables: None,
    flags: None,
    guild_id: None,
    hook: false,
    icon: None,
    id: "null".to_string(),
    name: String::new(),
    publishers: None,
    rpc_origins: None,
    splash: None,
    third_party_skus: None,
    type_field: None,
    verify_key: None,
    primary_sku_id: None,
    slug: None,
    aliases: None,
    overlay: None,
    overlay_compatibility_hook: None,
    privacy_policy_url: None,
    terms_of_service_url: None,
    eula_id: None,
    deeplink_uri: None,
    tags: None,
    pid: None,
    timestamp: None,
  })
}

fn build_ac_patterns(detectables: &[Arc<DetectableActivity>]) -> (AhoCorasick, Vec<[usize; 2]>) {
  let mut exe_patterns: Vec<String> = Vec::new();
  let mut exe_indexes: Vec<[usize; 2]> = Vec::new();

  for (activity_index, activity) in detectables.iter().enumerate() {
    if let Some(executables) = &activity.executables {
      for (exe_index, executable) in executables.iter().enumerate() {
        if executable.is_launcher {
          continue;
        }

        // Make paths consistent, and fix some additional checks
        let mut exec_name = executable.name.replace('\\', "/").to_lowercase();

        // Checks adapted from arrpc, remain the '>' in DetectableActivity for later argument checks
        if exec_name.starts_with(">") {
          exec_name.replace_range(0..1, "/");
        } else if !exec_name.starts_with("/") {
          exec_name.insert(0, '/');
        }

        exe_patterns.push(exec_name.chars().rev().collect::<String>());
        exe_indexes.push([activity_index, exe_index]);
      }
    }
  }

  (AhoCorasick::new(exe_patterns).unwrap(), exe_indexes)
}

#[cfg(test)]
mod tests {
  use super::*;

  fn detectable(name: &str, executable: &str) -> DetectableActivity {
    serde_json::from_value(serde_json::json!({
      "bot_public": true,
      "bot_require_code_grant": false,
      "description": "",
      "executables": [{
        "is_launcher": false,
        "name": executable,
        "os": "win32"
      }],
      "name": name,
      "flags": 0,
      "hook": true,
      "id": "activity-id",
      "type": 1
    }))
    .unwrap()
  }

  #[test]
  fn publishes_incremental_start_and_exit_transitions() {
    let (sender, receiver) = mpsc::channel();
    let server = ProcessServer::new(
      vec![Arc::new(detectable("Test Game", "game.exe"))],
      sender,
      ProcessEventListeners::default(),
    );

    server
      .process_started(41, r"C:\Games\game.exe".to_string(), None)
      .unwrap();
    let first = receiver.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(first.activity.id, "activity-id");
    assert_eq!(first.activity.pid, Some(41));

    server
      .process_started(41, r"C:\Games\game.exe".to_string(), None)
      .unwrap();
    assert!(receiver.try_recv().is_err());

    server
      .process_started(42, r"C:\Games\game.exe".to_string(), None)
      .unwrap();
    assert!(receiver.try_recv().is_err());

    server.process_exited(41);
    let replacement = receiver.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(replacement.activity.id, "activity-id");
    assert_eq!(replacement.activity.pid, Some(42));

    server.process_exited(42);
    let cleared = receiver.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(cleared.activity.id, "null");
  }

  #[test]
  fn ignores_non_matching_processes() {
    let (sender, receiver) = mpsc::channel();
    let server = ProcessServer::new(
      vec![Arc::new(detectable("Test Game", "game.exe"))],
      sender,
      ProcessEventListeners::default(),
    );

    server
      .process_started(99, r"C:\Windows\notepad.exe".to_string(), None)
      .unwrap();

    assert!(receiver.try_recv().is_err());
  }
}
