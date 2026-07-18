# RTC diagnostics

RTC diagnostics are disabled by default. To capture local samples while reproducing a stream or call issue, add this to Acord's `config.json` and restart the app:

```json
{
  "rtc_diagnostics": true
}
```

Samples are written every 15 seconds to `logs/latest.log`. Each `[RTC diagnostics]` entry is JSON containing aggregate WebRTC counters and rates: video stream counts, inbound and outbound bitrate, packet loss, decoded/dropped frames, round-trip time, and jitter.

The entry also records Acord's own CPU and memory usage, whether hardware acceleration was disabled in configuration, and whether the Linux DMA-BUF renderer was disabled. It does not record call IDs, participants, IP addresses, URLs, audio, video, or data from Discord messages.

This is diagnostic data, not a complete GPU profiler: browser/WebKit child-process GPU usage is not available through the cross-platform Tauri API. Disable the option after capturing a reproduction.
