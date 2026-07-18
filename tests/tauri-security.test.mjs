import assert from 'node:assert/strict'
import { readFileSync, readdirSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import test from 'node:test'

const repositoryRoot = dirname(dirname(fileURLToPath(import.meta.url)))

function read(relativePath) {
  return readFileSync(join(repositoryRoot, relativePath), 'utf8')
}

function readTypeScriptTree(relativeDirectory) {
  const directory = join(repositoryRoot, relativeDirectory)

  return readdirSync(directory, { withFileTypes: true })
    .flatMap(entry => {
      const relativePath = join(relativeDirectory, entry.name)
      if (entry.isDirectory()) return readTypeScriptTree(relativePath)
      return entry.name.endsWith('.ts') ? [read(relativePath)] : []
    })
    .join('\n')
}

const capability = JSON.parse(read('src-tauri/capabilities/default.json'))
const tauriConfig = JSON.parse(read('src-tauri/tauri.conf.json'))
const packageJson = JSON.parse(read('package.json'))
const injectionSource = readTypeScriptTree('src-tauri/injection')

test('keeps the global Tauri API disabled for remote content', () => {
  assert.equal(tauriConfig.app.withGlobalTauri, false)
  assert.doesNotMatch(injectionSource, /window\.__TAURI__/)
  assert.equal(packageJson.dependencies['@tauri-apps/api'], '^2.11.1')
})

test('limits the remote capability to supported Discord origins and the main window', () => {
  assert.equal(capability.local, false)
  assert.deepEqual(capability.windows, ['main'])
  assert.deepEqual(capability.remote.urls, [
    'https://discord.com/*',
    'https://canary.discord.com/*',
    'https://ptb.discord.com/*'
  ])
  assert.ok(capability.remote.urls.every(url => !url.includes('*.discord.com')))
})

test('keeps privileged plugins outside the remote capability', () => {
  const forbiddenPermissionPrefixes = [
    'autostart:',
    'deep-link:',
    'http:',
    'process:',
    'shell:',
    'window-state:'
  ]

  for (const permission of capability.permissions) {
    assert.equal(typeof permission, 'string')
    assert.ok(
      forbiddenPermissionPrefixes.every(prefix => !permission.startsWith(prefix)),
      `${permission} must not be granted to remote content`
    )
  }

  const cargoManifest = read('src-tauri/Cargo.toml')
  assert.doesNotMatch(cargoManifest, /tauri-plugin-(?:http|shell)/)
})

test('grants exactly the app commands used by the injected bundle', () => {
  const invokedCommands = new Set()
  const invokePattern = /\binvoke(?:<[^>]+>)?\(\s*['"]([^'"]+)['"]/g

  for (const match of injectionSource.matchAll(invokePattern)) {
    invokedCommands.add(match[1])
  }

  const expectedPermissions = [...invokedCommands]
    .map(command => `allow-${command.replaceAll('_', '-')}`)
    .sort()
  const grantedAppPermissions = capability.permissions
    .filter(permission => permission.startsWith('allow-'))
    .sort()

  assert.deepEqual(grantedAppPermissions, expectedPermissions)
  assert.deepEqual(
    capability.permissions.filter(permission => permission.startsWith('core:')).sort(),
    ['core:event:allow-listen', 'core:event:allow-unlisten']
  )
})
