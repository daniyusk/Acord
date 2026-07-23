use wgpu::{
  BackendOptions, Backends, DeviceType, GlBackendOptions, Instance, InstanceDescriptor,
  InstanceFlags,
};

pub fn configure_before_creation(_disable_hardware_accel: bool) {
  // Disable DMA rendering on Linux + NVIDIA systems.
  // See https://github.com/SpikeHD/Dorion/issues/237 and https://github.com/tauri-apps/tauri/issues/9304.
  let instance = Instance::new(&InstanceDescriptor {
    flags: InstanceFlags::empty(),
    backends: Backends::GL | Backends::VULKAN,
    memory_budget_thresholds: Default::default(),
    backend_options: BackendOptions {
      gl: GlBackendOptions::default(),
      dx12: Default::default(),
      noop: Default::default(),
    },
  });

  for adapter in instance.enumerate_adapters(Backends::all()) {
    let info = adapter.get_info();

    if matches!(
      info.device_type,
      DeviceType::DiscreteGpu | DeviceType::IntegratedGpu | DeviceType::VirtualGpu
    ) && info.name.contains("NVIDIA")
    {
      crate::log!("NVIDIA GPU detected, disabling DMA");
      unsafe { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1") };
    }
  }
}

pub fn configure_after_creation(window: &tauri::WebviewWindow) {
  use tauri::Manager;
  use webkit2gtk::{HardwareAccelerationPolicy, SettingsExt, WebViewExt};

  window
    .with_webview(move |webview| {
      let config = crate::config::get_config();
      let webview = webview.inner();
      let settings = WebViewExt::settings(&webview).unwrap_or_default();

      if config.disable_hardware_accel.unwrap_or(false) {
        settings.set_hardware_acceleration_policy(HardwareAccelerationPolicy::Never);
      }
    })
    .unwrap_or_else(|_| crate::log!("Failed to disable hardware acceleration"));
}
