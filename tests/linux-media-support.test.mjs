import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import test from 'node:test'

const root = new URL('../', import.meta.url)
const read = (path) => readFile(new URL(path, root), 'utf8')

test('keeps Linux WebKitGTK media support explicitly experimental', async () => {
  const [readme, policy, linux] = await Promise.all([
    read('README.md'),
    read('docs/linux-media-support.md'),
    read('src-tauri/src/functionality/configure/linux.rs'),
  ])

  assert.match(readme, /WebKitGTK on Linux and are currently \*\*experimental\*\*/)
  assert.match(linux, /voice, video, and screen sharing use WebKitGTK and are experimental/)
  assert.match(policy, /Tauri \+ WebKitGTK/)
  assert.match(policy, /Electron distribution for Linux/)
  assert.match(policy, /CEF host for Linux/)
  assert.match(policy, /Node integration disabled/)
  assert.match(policy, /context isolation enabled/)
})
