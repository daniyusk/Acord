import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import test from 'node:test'

const root = new URL('../', import.meta.url)
const read = (path) => readFile(new URL(path, root), 'utf8')

test('keeps RTC diagnostics disabled until explicitly enabled', async () => {
  const [config, preinject] = await Promise.all([
    read('src-tauri/src/config.rs'),
    read('src-tauri/injection/preinject.ts'),
  ])

  assert.match(config, /pub rtc_diagnostics: Option<bool>/)
  assert.match(config, /rtc_diagnostics: Option::from\(false\)/)
  assert.match(preinject, /rtc_diagnostics === true/)
})

test('records only bounded, local RTC and process metrics', async () => {
  const [diagnostics, injection, capability, manifest, entrypoint] = await Promise.all([
    read('src-tauri/src/functionality/diagnostics.rs'),
    read('src-tauri/injection/shared/rtc_diagnostics.ts'),
    read('src-tauri/capabilities/default.json').then(JSON.parse),
    read('src-tauri/build.rs'),
    read('src-tauri/src/main.rs'),
  ])

  assert.match(injection, /connection\.getStats\(\)/)
  assert.match(injection, /SAMPLE_INTERVAL_MS = 15_000/)
  assert.doesNotMatch(injection, /\b(?:fetch|XMLHttpRequest|WebSocket)\b/)
  assert.match(diagnostics, /const MIN_SAMPLE_INTERVAL: Duration = Duration::from_secs\(10\)/)
  assert.match(diagnostics, /fn validate\(&self\)/)
  assert.match(diagnostics, /hardware_acceleration_disabled/)
  assert.match(diagnostics, /dma_buf_renderer_disabled/)
  assert.ok(capability.permissions.includes('allow-record-rtc-diagnostics'))
  assert.match(manifest, /"record_rtc_diagnostics"/)
  assert.match(entrypoint, /functionality::diagnostics::record_rtc_diagnostics/)
})
