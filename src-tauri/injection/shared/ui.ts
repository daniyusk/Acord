import { timeout, waitForDom } from './util'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

export function safemodeTimer(elm: HTMLDivElement) {
  setTimeout(() => {
    elm.classList.add('show')
  }, 10000)

  const tmpKeydown = (evt: KeyboardEvent) => {
    // If loading container doesn't exist, we made it through and should stop watching key events
    if (!document.querySelector('#loadingContainer')) {
      document.removeEventListener('keydown', tmpKeydown)
      return
    }

    // If spacebar, remove #loadingContainer
    if (evt.code === 'Space') {
      document.querySelector('#loadingContainer')?.remove()
    }

    // If F, open plugins folder
    if (evt.code === 'KeyF') {
      void invoke<void>('open_themes')
    }

    // If S, relaunch once in safe mode.
    if (evt.code === 'KeyS') {
      void invoke('restart_in_safemode')
    }
  }

  document.addEventListener('keydown', tmpKeydown)
}

export async function typingAnim() {
  const title = document.querySelector('#title')

  if (!title) return

  for (const letter of 'Acord'.split('')) {
    title.innerHTML = title.innerHTML.replace('|', '') + letter + '|'

    await timeout(100)
  }

  // Once the "typing" is done, blink the cursor
  let cur = true

  const interval = setInterval(() => {
    if (!document.querySelector('#loadingContainer') || !document.querySelector('#title')) {
      clearInterval(interval)
      return
    }

    if (cur) {
      cur = false

      title.innerHTML = title.innerHTML.replace('|', '&nbsp;')
      return
    }

    cur = true

    title.innerHTML = title.innerHTML.replace(/&nbsp;$/, '|')
  }, 500)
}

export async function applyExtraCSS() {
  const css = await invoke<string>('get_extra_css')
  const style = document.createElement('style')

  style.innerHTML = css
  style.id = 'dorion-extra-css'

  // Append some background-transparenting css if blur_css is true
  if (window.__DORION_CONFIG__.blur !== 'none' && window.__DORION_CONFIG__.blur_css) {
    style.innerHTML += `
      * {
        background: transparent !important;
      }
    `
  }

  document.body.appendChild(style)
}

export async function extraCssChangeWatch() {
  await waitForDom()

  const style = document.createElement('style')
  style.id = 'dorion-os-accent'

  const elm = document.body.appendChild(style)

  // Get the initial color
  const initial = await invoke<string>('get_os_accent')
  const setAccentColor = (color: string) => {
    elm.innerText = `html { --os-accent-color: ${color} !important; }`
  }

  setAccentColor(initial)

  void listen<string>('os_accent_update', (event) => {
    setAccentColor(event.payload)
  })
}
