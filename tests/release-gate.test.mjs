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

  assert.match(packageJson.scripts['test:release'], /pnpm test:rust/)
  assert.equal(packageJson.scripts['test:runtime'], 'node scripts/smoke-test-binary.mjs')
  assert.ok(workflow.indexOf('pnpm build:js') < workflow.indexOf('Run release test suite'))
  assert.ok(workflow.indexOf('Run release test suite') < workflow.indexOf('- name: Build'))
  assert.match(workflow, /pnpm test:runtime -- "src-tauri\/target\/\$\{\{ matrix\.config\.target \}\}\/release\/Acord\.exe"/)
  assert.match(workflow, /pnpm test:runtime -- "src-tauri\/target\/\$\{\{ matrix\.config\.target \}\}\/release\/Acord"/)
})

test('the build workflow uses stable Rust and declares its supported minimum', async () => {
  const [manifest, workflow] = await Promise.all([
    readProjectFile('src-tauri/Cargo.toml'),
    readProjectFile('.github/workflows/build.yml'),
  ])

  assert.match(manifest, /^rust-version = "1\.89"$/m)
  assert.match(workflow, /toolchain: stable/)
})

test('a draft release only consumes a successful build for its checked-out commit', async () => {
  const workflow = await readProjectFile('.github/workflows/create-release.yml')

  assert.match(workflow, /--branch \$BRANCH --commit \$COMMIT/)
  assert.match(workflow, /BRANCH: \$\{\{ github\.ref_name \}\}/)
  assert.match(workflow, /COMMIT: \$\{\{ github\.sha \}\}/)
})
