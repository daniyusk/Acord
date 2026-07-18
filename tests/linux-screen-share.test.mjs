import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import test from 'node:test'

const root = new URL('../', import.meta.url)
const read = (path) => readFile(new URL(path, root), 'utf8')

test('checks PipeWire and XDG Portal prerequisites only with RTC diagnostics enabled', async () => {
  const [linux, configure, docs] = await Promise.all([
    read('src-tauri/src/functionality/linux_screen_share.rs'),
    read('src-tauri/src/functionality/configure/linux.rs'),
    read('docs/rtc-diagnostics.md'),
  ])

  assert.match(configure, /rtc_diagnostics\.unwrap_or\(false\)/)
  assert.match(configure, /log_linux_screen_share_diagnostics\(\)/)
  assert.match(linux, /XDG_RUNTIME_DIR/)
  assert.match(linux, /pipewire-0/)
  assert.match(linux, /xdg-desktop-portal\.service/)
  assert.match(linux, /xdg-desktop-portal"\)\.join\("portals/)
  assert.match(linux, /Command::new\("systemctl"\)/)
  assert.doesNotMatch(linux, /(?:sh|bash)\s+-c/)
  assert.match(docs, /D-Bus activated/)
})
