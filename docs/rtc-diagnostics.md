# RTC diagnostics

RTC diagnostics are disabled by default. To capture local samples while reproducing a stream or call issue, add this to Acord's `config.json` and restart the app:

```json
{
  "rtc_diagnostics": true
}
```

Samples are written every 15 seconds to `logs/latest.log`. Each `[RTC diagnostics]` entry is JSON containing aggregate WebRTC counters and rates: video stream counts, inbound and outbound bitrate, packet loss, decoded/dropped frames, round-trip time, and jitter.

The entry also records Acord's own CPU and memory usage, whether hardware acceleration was disabled in configuration, and whether the Linux DMA-BUF renderer was disabled. It does not record call IDs, participants, IP addresses, URLs, audio, video, or data from Discord messages.

On Linux, enabling this option also writes one `[Linux screen share]` report at startup. It checks the graphical session type, XDG runtime and D-Bus sockets, the PipeWire socket, the installed portal backend files, and the user-service state of `pipewire` and `xdg-desktop-portal`. When a prerequisite is unavailable, the log includes a local, actionable recommendation. A portal service can be inactive before the first request because it is normally D-Bus activated; this alone is not treated as a failure.

This is diagnostic data, not a complete GPU profiler: browser/WebKit child-process GPU usage is not available through the cross-platform Tauri API. Disable the option after capturing a reproduction.
