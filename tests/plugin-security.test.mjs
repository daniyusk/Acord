import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import test from 'node:test'

const repositoryRoot = new URL('../', import.meta.url)

const readRepositoryFile = (path) =>
  readFile(new URL(path, repositoryRoot), 'utf8')

test('keeps a restrictive CSP for Acord-owned content', async () => {
  const config = JSON.parse(await readRepositoryFile('src-tauri/tauri.conf.json'))
  const csp = config.app.security.csp

  assert.equal(typeof csp, 'string')
  assert.match(csp, /default-src 'self'/)
  assert.match(csp, /object-src 'none'/)
  assert.match(csp, /base-uri 'none'/)
  assert.match(csp, /frame-ancestors 'none'/)
  assert.doesNotMatch(csp, /unsafe-eval/)
  assert.equal(config.app.security.freezePrototype, true)
})

test('does not remove Discord CSP and keeps third-party code opt-in', async () => {
  const [manifestSource, preinjectSource, configSource, clientModSource] = await Promise.all([
    readRepositoryFile('src-tauri/extension/manifest.json'),
    readRepositoryFile('src-tauri/injection/preinject.ts'),
    readRepositoryFile('src-tauri/src/config.rs'),
    readRepositoryFile('src-tauri/src/injection/client_mod.rs'),
  ])
  const manifest = JSON.parse(manifestSource)

  assert.deepEqual(manifest.permissions, ['privacy'])
  assert.equal(manifest.declarative_net_request, undefined)
  assert.match(configSource, /client_mods: Option::from\(vec!\[\]\)/)
  assert.match(configSource, /client_plugins: Option::from\(false\)/)
  assert.match(preinjectSource, /const clientPluginsEnabled = window\.__DORION_CONFIG__\.client_plugins === true/)
  assert.match(preinjectSource, /if \(clientPluginsEnabled\) window\.SHELTER_INJECTOR_PLUGINS/)
  assert.doesNotMatch(clientModSource, /adding to config|FALLBACK/)
})
