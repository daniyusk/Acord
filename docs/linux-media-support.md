# Linux voice and video support

## Support level

Voice, video, and screen sharing on Linux are **experimental** in Acord. The Linux Tauri runtime uses WebKitGTK, so media behavior depends on the installed WebKitGTK, GStreamer, graphics driver, PipeWire, and XDG Portal stack. This is not equivalent to Discord's Chromium-based desktop client.

When investigating a failure, first enable [RTC diagnostics](rtc-diagnostics.md). On Linux this records a local report for PipeWire, XDG Portal, and the selected graphical session. Do not report a stream as generally supported solely because it works on one distribution or desktop environment.

## Current support policy

- Keep the existing Tauri/WebKitGTK path for general application use.
- Mark voice, video, and screen sharing as experimental in documentation and startup logs.
- Fix issues that have a reproducible report, logs with no sensitive content, and a regression test where practical.
- Do not enable a global WebKitGTK GPU workaround unless it is proven necessary: these workarounds can disable faster rendering paths.

Tauri documents known Linux WebKitGTK/DMABUF graphics failures and their trade-offs: <https://v2.tauri.app/develop/debug/linux-graphics/>. Tauri also documents that its Linux webview is WebKitGTK rather than a bundled Chromium runtime: <https://v2.tauri.app/concept/process-model/>.

## Chromium backend evaluation

This is an architectural evaluation, not an in-place backend switch. Tauri's Linux webview is WebKitGTK, so Chromium/CEF cannot be selected as a configuration value for the current app; either option requires a distinct Linux host and a migration of windowing, IPC, permissions, packaging, and release operations.

| Option | Compatibility expectation | Cost and risk | Decision |
| --- | --- | --- | --- |
| Keep Tauri + WebKitGTK | Lowest footprint; media remains distro-dependent. | Existing code path; experimental media contract. | Keep as the current path. |
| Electron distribution for Linux | Best candidate for Chromium-oriented Discord media parity. Electron embeds Chromium and Node.js. | Larger package and multi-process runtime; rewrite native integration and preserve the security boundary. | Preferred proof of concept. |
| CEF host for Linux | Chromium rendering with a native host. | Requires maintaining custom native bindings, CEF distribution, updates, and an application host. | Do not start here. |

Electron documents its Chromium/Node embedding and multi-process model at <https://www.electronjs.org/docs/latest/> and <https://www.electronjs.org/docs/latest/tutorial/process-model>. CEF describes itself as a framework for embedding Chromium-based browsers: <https://bitbucket.org/chromiumembedded/cef>.

## Proposed Electron proof of concept

Build a Linux-only proof of concept outside the release path. Reuse only the safe injected-client logic; do not give Discord's remote origin direct Node.js, filesystem, shell, or unrestricted IPC access.

The proof of concept passes only if it demonstrates all of the following on current Arch, Ubuntu, and Fedora images, with both a Wayland and an X11 session where available:

1. Join and leave voice, camera, and a PipeWire portal screen share successfully.
2. View a remote stream at stable quality for a sustained manual session.
3. Preserve profile isolation, deep links, notifications, and the least-privilege plugin model.
4. Keep the remote Discord renderer sandboxed, with Node integration disabled, context isolation enabled, a narrow preload bridge, restrictive CSP, and sender validation for IPC.
5. Compare startup time, idle memory, active call CPU, package size, crash rate, and support burden against the WebKitGTK build before considering a release.

Electron's security guidance specifically requires disabling Node integration and enabling context isolation for remote content, as well as using sandboxing, CSP, and validated IPC: <https://www.electronjs.org/docs/latest/tutorial/security>.

Until those gates pass, Acord will not claim production-grade Linux voice/video compatibility or ship a Chromium/CEF backend.
