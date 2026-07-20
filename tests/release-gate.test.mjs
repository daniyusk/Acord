import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import test from 'node:test'

const root = new URL('..', import.meta.url)

async function readProjectFile(path) {
  return readFile(new URL(path, root), 'utf8')
}

test('the build workflow runs release checks before packaging artifacts', async () => {
  const [packageJson, workflow] = await Promise.all([
    readProjectFile('package.json').then(JSON.parse),
    readProjectFile('.github/workflows/build.yml'),
  ])

  assert.equal(packageJson.scripts['prepare:generated-assets'], 'pnpm shupdate && pnpm build:js')
  assert.equal(packageJson.scripts.verify, 'pnpm prepare:generated-assets && pnpm test:release')
  assert.match(packageJson.scripts['test:release'], /pnpm test:rust/)
  assert.equal(packageJson.scripts['test:runtime'], 'node scripts/smoke-test-binary.mjs')
  assert.ok(workflow.indexOf('pnpm verify') > workflow.indexOf('pnpm install'))
  assert.match(workflow, /libsoup-3\.0-dev/)
  assert.ok(workflow.indexOf('Run self-contained verification') < workflow.indexOf('- name: Build'))
  assert.match(workflow, /pnpm test:runtime -- "src-tauri\/target\/\$\{\{ matrix\.config\.target \}\}\/release\/Acord\.exe"/)
  assert.match(workflow, /pnpm test:runtime -- "src-tauri\/target\/\$\{\{ matrix\.config\.target \}\}\/release\/Acord"/)
})

test('the build workflow uses the pinned Rust toolchain and declares its supported minimum', async () => {
  const [manifest, workflow, toolchain] = await Promise.all([
    readProjectFile('src-tauri/Cargo.toml'),
    readProjectFile('.github/workflows/build.yml'),
    readProjectFile('rust-toolchain.toml'),
  ])

  assert.match(manifest, /^rust-version = "1\.89"$/m)
  assert.match(toolchain, /^channel = "1\.97\.1"$/m)
  assert.match(toolchain, /^components = \["clippy", "rustfmt"\]$/m)
  assert.doesNotMatch(workflow, /toolchain:/)
})

test('a draft release only consumes a successful build for its checked-out commit', async () => {
  const workflow = await readProjectFile('.github/workflows/create-release.yml')

  assert.match(workflow, /--branch \$BRANCH --commit \$COMMIT/)
  assert.match(workflow, /BRANCH: \$\{\{ github\.ref_name \}\}/)
  assert.match(workflow, /COMMIT: \$\{\{ github\.sha \}\}/)
})
