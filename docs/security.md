# Security model

## Trust boundaries

Acord's native backend, bundled injection code, and the official Tauri plugins are trusted application code. Discord, themes, client mods, local plugins, and plugin URLs are not trusted application code.

Themes can alter the interface and request remote assets. Client mods and plugins execute JavaScript in the Discord webview; they can read and change Discord data visible to that webview. They are therefore disabled by default.

Enabling `client_plugins` or adding a client mod is an explicit trust decision by the user. Only enable code from a source you control or have reviewed. Safe mode is the recovery path for a broken or untrusted extension.

## Native capabilities

Remote Discord content receives only the commands listed in `src-tauri/capabilities/default.json`. Acord does not expose `window.__TAURI__`, and does not grant HTTP, Shell, process, autostart, deep-link, or window-state permissions to remote content.

This is a reduction of native privilege, not a sandbox for enabled plugins. Plugins share the Discord webview, so an enabled plugin must be treated as privileged web content. Acord must not describe third-party plugins as isolated or safe by default.

## Content Security Policy

The Tauri CSP protects HTML that Acord serves through its own protocol. It prohibits plugin-controlled scripts, plugin-controlled frames, form submissions, and object embedding in that local application surface. The Windows WebView extension does not remove Discord's CSP.

Discord is remote content and controls its own response headers. A CSP in `tauri.conf.json` cannot turn arbitrary JavaScript executing in Discord's webview into a sandbox. The mitigation is to keep client mods and plugins opt-in, avoid global native APIs, and keep their Tauri permissions minimal.

## Maintenance rules

- Do not reintroduce a rule that removes or weakens Discord's CSP.
- Do not enable client mods or plugins in the default configuration.
- Keep `test:security` passing whenever Tauri permissions, CSP, plugin loading, or extension manifests change.
