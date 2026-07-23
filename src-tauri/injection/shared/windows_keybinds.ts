import { invoke } from '@tauri-apps/api/core'

const currentlyPressed = new Set<string>()

const MODIFIER_CODES = new Set([
  'ControlLeft',
  'ControlRight',
  'ShiftLeft',
  'ShiftRight',
  'AltLeft',
  'AltRight',
  'MetaLeft',
  'MetaRight'
])

const FUNCTION_KEY_CODES = new Set([
  'F1', 'F2', 'F3', 'F4', 'F5', 'F6', 'F7', 'F8', 'F9', 'F10', 'F11', 'F12'
])

export async function initWindowsKeybinds() {
  try {
    const platform = await invoke<string>('get_platform')
    if (platform !== 'windows') return
  } catch {
    return
  }

  document.addEventListener('keydown', handleKeyDown)
  document.addEventListener('keyup', handleKeyUp)
}

function shouldTriggerIpc(event: KeyboardEvent): boolean {
  if (event.ctrlKey || event.altKey || event.shiftKey || event.metaKey) {
    return true
  }

  if (FUNCTION_KEY_CODES.has(event.code)) {
    return true
  }

  for (const code of currentlyPressed) {
    if (MODIFIER_CODES.has(code) || FUNCTION_KEY_CODES.has(code)) {
      return true
    }
  }

  return false
}

function handleKeyDown(event: KeyboardEvent) {
  currentlyPressed.add(event.code)

  if (!shouldTriggerIpc(event)) return

  const keys = Array.from(currentlyPressed).map(code => ({
    name: getDisplayName(code),
    code: code
  }))

  invoke<void>('trigger_keys_pressed', { keys, pressed: true }).catch(() => {})
}

function handleKeyUp(event: KeyboardEvent) {
  if (shouldTriggerIpc(event)) {
    const keys = Array.from(currentlyPressed).map(code => ({
      name: getDisplayName(code),
      code: code
    }))

    invoke<void>('trigger_keys_pressed', { keys, pressed: false }).catch(() => {})
  }

  currentlyPressed.delete(event.code)
}

function getDisplayName(code: string): string {
  const names: Record<string, string> = {
    'ControlLeft': 'Ctrl', 'ControlRight': 'Ctrl',
    'ShiftLeft': 'Shift', 'ShiftRight': 'Shift',
    'AltLeft': 'Alt', 'AltRight': 'Alt',
    'MetaLeft': 'Meta', 'MetaRight': 'Meta',
    'Space': 'Space'
  }

  if (code.startsWith('Key') && code.length === 4) {
    return code.slice(3)
  }

  return names[code] || code
}

