import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import test from 'node:test'

const repositoryRoot = new URL('../', import.meta.url)

const readRepositoryFile = (path) =>
  readFile(new URL(path, repositoryRoot), 'utf8')

test('the webview receives only the Tauri permissions it uses', async () => {
  const capability = JSON.parse(
    await readRepositoryFile('src-tauri/capabilities/default.json')
  )
  const permissionIdentifiers = capability.permissions.map((permission) =>
    typeof permission === 'string' ? permission : permission.identifier
  )

  assert.deepEqual(permissionIdentifiers.slice(0, 4), [
    'core:event:allow-listen',
    'core:event:allow-unlisten',
    'opener:allow-open-url',
    'opener:allow-default-urls',
  ])
  assert.ok(permissionIdentifiers.every(permission =>
    !permission.startsWith('http:') && !permission.startsWith('shell:')
  ))
})

test('the unused process plugin stays unregistered', async () => {
  const [cargoManifest, applicationEntryPoint] = await Promise.all([
    readRepositoryFile('src-tauri/Cargo.toml'),
    readRepositoryFile('src-tauri/src/main.rs'),
  ])

  assert.doesNotMatch(cargoManifest, /^tauri-plugin-process\s*=/m)
  assert.doesNotMatch(applicationEntryPoint, /tauri_plugin_process/)
})
