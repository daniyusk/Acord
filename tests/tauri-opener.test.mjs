import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import test from 'node:test'

const repositoryRoot = new URL('../', import.meta.url)

const readRepositoryFile = (path) =>
  readFile(new URL(path, repositoryRoot), 'utf8')

test('external links use the official Tauri opener bindings', async () => {
  const [packageJsonSource, recreateSource] = await Promise.all([
    readRepositoryFile('package.json'),
    readRepositoryFile('src-tauri/injection/shared/recreate.ts'),
  ])
  const packageJson = JSON.parse(packageJsonSource)
  const openerVersion = packageJson.dependencies?.['@tauri-apps/plugin-opener']

  assert.match(openerVersion, /^\d+\.\d+\.\d+$/)
  assert.equal(packageJson.dependencies?.['@tauri-apps/plugin-shell'], undefined)
  assert.match(
    recreateSource,
    /import \{ openUrl \} from '@tauri-apps\/plugin-opener'/
  )
  assert.match(recreateSource, /\bopenUrl\(urlStr\)/)
  assert.doesNotMatch(recreateSource, /__TAURI__\.shell/)
})

test('the Rust backend uses the registry opener plugin and scoped permissions', async () => {
  const [cargoManifest, cargoLock, capabilitySource, applicationEntryPoint] =
    await Promise.all([
      readRepositoryFile('src-tauri/Cargo.toml'),
      readRepositoryFile('src-tauri/Cargo.lock'),
      readRepositoryFile('src-tauri/capabilities/default.json'),
      readRepositoryFile('src-tauri/src/main.rs'),
    ])
  const capability = JSON.parse(capabilitySource)
  const permissionIdentifiers = capability.permissions.map((permission) =>
    typeof permission === 'string' ? permission : permission.identifier
  )

  assert.match(
    cargoManifest,
    /^tauri-plugin-opener\s*=\s*"\d+\.\d+\.\d+"$/m
  )
  assert.doesNotMatch(cargoManifest, /^tauri-plugin-shell\s*=/m)
  assert.match(
    cargoLock,
    /name = "tauri-plugin-opener"\r?\nversion = "\d+\.\d+\.\d+"\r?\nsource = "registry\+https:\/\/github\.com\/rust-lang\/crates\.io-index"/
  )
  assert.doesNotMatch(cargoLock, /SpikeHD\/plugins-workspace/)
  assert.match(applicationEntryPoint, /tauri_plugin_opener::init\(\)/)
  assert.doesNotMatch(applicationEntryPoint, /tauri_plugin_shell/)
  assert.ok(permissionIdentifiers.includes('opener:allow-open-url'))
  assert.ok(permissionIdentifiers.includes('opener:allow-default-urls'))
  assert.ok(permissionIdentifiers.every((permission) => !permission.startsWith('shell:')))
})
