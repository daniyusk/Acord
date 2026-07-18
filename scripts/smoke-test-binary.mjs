import { spawn } from 'node:child_process'
import { access } from 'node:fs/promises'
import process from 'node:process'

const [binary] = process.argv.slice(2)

if (!binary) {
  throw new Error('Usage: pnpm test:runtime -- <path-to-binary>')
}

await access(binary)

const result = await new Promise((resolve, reject) => {
  const child = spawn(binary, ['--help'], {
    stdio: ['ignore', 'pipe', 'pipe'],
    windowsHide: true,
  })
  let output = ''
  const timeout = setTimeout(() => {
    child.kill()
    reject(new Error(`Timed out while running ${binary} --help`))
  }, 15_000)

  child.stdout.on('data', (chunk) => {
    output += chunk
  })
  child.stderr.on('data', (chunk) => {
    output += chunk
  })
  child.on('error', (error) => {
    clearTimeout(timeout)
    reject(error)
  })
  child.on('close', (code, signal) => {
    clearTimeout(timeout)
    resolve({ code, signal, output })
  })
})

if (result.code !== 0 || result.signal || !/Usage:/i.test(result.output)) {
  throw new Error(
    `${binary} --help did not complete successfully (code: ${result.code}, signal: ${result.signal})\n${result.output}`,
  )
}
