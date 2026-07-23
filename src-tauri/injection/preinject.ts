import { proxyXHR, proxyAddEventListener, proxyOpen, proxyNotification } from './shared/recreate'
import { extraCssChangeWatch, safemodeTimer, typingAnim } from './shared/ui'
import { cssSanitize, fetchImage, isJson, waitForApp, waitForElm, saferEval } from './shared/util'
import { waitForElmEx } from './shared/wait_elm'
import { startRtcDiagnostics } from './shared/rtc_diagnostics'
import { invoke } from '@tauri-apps/api/core'
import { listen, TauriEvent } from '@tauri-apps/api/event'

type AppConfig = Record<string, unknown>

// Let's expose some stuff for use in plugins and such
window.Dorion = {
  util: {
    cssSanitize,
    isJson,
    fetchImage,
    waitForApp,
    waitForElm,
    waitForElmEx,
  },
  shouldShowUnreadBadge: false
}

async function readConfig(): Promise<AppConfig> {
  const serializedConfig = await invoke<string>('read_config_file')

  if (serializedConfig && isJson(serializedConfig)) {
    return JSON.parse(serializedConfig) as AppConfig
  }

  const defaultConfig = await invoke<AppConfig>('default_config')
  await invoke<void>('write_config_file', {
    contents: JSON.stringify(defaultConfig)
  })

  return defaultConfig
}

(async () => {
  // if we are in an iframe we don't really need to load anything, else we bork whatever is inside
  if (window.self !== window.top) {
    console.log('Stopping here, we are in an iframe!')
    return
  }

  proxyXHR()
  proxyAddEventListener()
  proxyNotification()

  console.log('Tauri modules initialized!')

  extraCssChangeWatch()
  proxyOpen()

  const platform = await invoke<string>('get_platform')
  document.documentElement.setAttribute('data-dorion-platform', platform)

  window.__DORION_CONFIG__ = await readConfig()

  if (window.__DORION_CONFIG__.rtc_diagnostics === true) {
    startRtcDiagnostics()
  }

  const clientPluginsEnabled = window.__DORION_CONFIG__.client_plugins === true
  console.log(window.__DORION_CONFIG__)
  const INJECTED_PLUGIN_OPTIONS = {
    isVisible: true,
    allowedActions: { toggle: clientPluginsEnabled },
    loaderName: 'Acord'
  }

  if (clientPluginsEnabled) window.SHELTER_INJECTOR_PLUGINS = {
    'Antitrack': ['https://yellowsink.github.io/shelter-plugins/antitrack/', {
      ...INJECTED_PLUGIN_OPTIONS,
      allowedActions: {
        toggle: true,
      }
    }],
    'Dorion Titlebar': ['https://spikehd.dev/shelter-plugins/dorion-titlebar/', INJECTED_PLUGIN_OPTIONS],
    'Dorion Settings': ['https://spikehd.dev/shelter-plugins/dorion-settings/', INJECTED_PLUGIN_OPTIONS],
    'Always Trust': ['https://spikehd.dev/shelter-plugins/always-trust/', INJECTED_PLUGIN_OPTIONS],
    'Dorion Notifications': ['https://spikehd.dev/shelter-plugins/dorion-notifications/', INJECTED_PLUGIN_OPTIONS],
    'Dorion Streamer Mode': ['https://spikehd.dev/shelter-plugins/dorion-streamer-mode/', INJECTED_PLUGIN_OPTIONS],
    'Dorion Updater': ['https://spikehd.dev/shelter-plugins/dorion-updater/', INJECTED_PLUGIN_OPTIONS],
    'Dorion PTT': ['https://spikehd.dev/shelter-plugins/dorion-ptt/', INJECTED_PLUGIN_OPTIONS],
    'Dorion Tray': ['https://spikehd.dev/shelter-plugins/dorion-tray/', INJECTED_PLUGIN_OPTIONS],
    'Dorion Fullscreen': ['https://spikehd.dev/shelter-plugins/dorion-fullscreen/', INJECTED_PLUGIN_OPTIONS],
    'Dorion Custom Keybinds': ['https://spikehd.dev/shelter-plugins/dorion-custom-keybinds/', INJECTED_PLUGIN_OPTIONS],
    'Dorion Helpers': ['https://spikehd.dev/shelter-plugins/dorion-helpers/', INJECTED_PLUGIN_OPTIONS],
    'Web Keybinds': ['https://spikehd.dev/shelter-plugins/web-keybinds/', {
      ...INJECTED_PLUGIN_OPTIONS,
      allowedActions: {
        toggle: true,
      }
    }],
  }

  init()
})()

async function init() {
  window.__DORION_CONFIG__ = await readConfig()

  window.Dorion.shouldShowUnreadBadge = window.__DORION_CONFIG__.unread_badge

  if (window.__DORION_CONFIG__.client_plugins === true) {
    await invoke<void>('load_plugins')
  }

  const version = await invoke<string>('app_version')

  await displayLoadingTop()

  // Start the safemode timer
  safemodeTimer(
    document.querySelector('#safemode') as HTMLDivElement
  )

  updateOverlay({
    subtitle: `Acord - v${version}`,
    midtitle: 'Localizing JS imports...'
  })

  typingAnim()

  // Discord Web depends on the `beforeunload` event being dispatched by the browser when
  // a tab is closed. However, this event is not triggered by the Webview so we need to
  // dispatch the `beforeunload` event ourselves.
  const dispatchBeforeUnload = () => {
    const event = new Event('beforeunload') as Event & { isTrustedOverwrite: boolean }
    event.isTrustedOverwrite = true
    window.dispatchEvent(event)
  }

  void listen('beforeunload', dispatchBeforeUnload)
  void listen(TauriEvent.WINDOW_CLOSE_REQUESTED, dispatchBeforeUnload)

  // Start the loading_log event listener
  const logUnlisten = await listen<string>('loading_log', (event) => {
    updateOverlay({
      logs: event.payload
    })
  })

  let themeJs = await handleClientModThemeInjection()
  themeJs += await handleThemeInjection()

  updateOverlay({
    midtitle: 'Getting injection JS...'
  })

  const injectionJs = await invoke<string>('get_injection_js', {
    themeJs,
  })

  saferEval(injectionJs)

  updateOverlay({
    midtitle: 'Done!'
  })

  // Remove loading container
  const loadingContainer = document.querySelector('#loadingContainer') as HTMLDivElement
  loadingContainer.style.opacity = '0'

  setTimeout(() => {
    loadingContainer?.remove()
  }, 200)

  // Unlisten from the log event
  logUnlisten()
}

/**
 * Nasty helper function _for updating the text on the overlay
 */
async function updateOverlay(toUpdate: Record<string, string>) {
  const midtitle = document.getElementById('midtitle')
  const subtitle = document.getElementById('subtitle')
  const safemode = document.getElementById('safemode')
  const logs = document.getElementById('logContainer')

  for (const [key, value] of Object.entries(toUpdate)) {
    if (key === 'midtitle' && midtitle) midtitle.innerHTML = value
    if (key === 'subtitle' && subtitle) subtitle.innerHTML = value
    if (key === 'safemode' && safemode) safemode.innerHTML = value
    if (key === 'logs' && logs) logs.innerHTML = value
  }
}

async function handleThemeInjection() {
  // This needs to exist for hot-switching to work
  const ts = document.createElement('style')
  ts.id = 'dorion-theme'
  document.body.appendChild(ts)

  if (!window.__DORION_CONFIG__?.themes || window.__DORION_CONFIG__?.themes.length === 0) return ''

  updateOverlay({
    midtitle: 'Loading theme CSS...'
  })

  // Get the initial theme
  const themeContents = await invoke<string>('get_themes').catch(e => console.error(e)) || ''

  updateOverlay({
    midtitle: 'Localizing CSS imports...'
  })

  // Create a "name" for the "theme" (or combo) based on the retrieved enabled theme list
  const themeNames = await invoke<string[]>('get_enabled_themes').catch(e => console.error(e)) || []
  // Gotta adhere to filename length restrictions
  const themeName = themeNames.join('').substring(0, 254)

  // Localize the imports. On windows this no longer does anything
  const localized = await invoke<string>('localize_imports', {
    css: themeContents,
    name: themeName
  })

  // This will use the DOM in a funky way to validate the css, then we make sure to fix up quotes
  const cleanContents = cssSanitize(localized)?.replaceAll('\\"', '\'')

  return `;(() => {
    const ts = document.querySelector('#dorion-theme')
    ts.textContent = \`
      ${cleanContents?.replace(/`/g, '\\`')
  // To this day I do not know why I need to do this
    .replace(/\\8/g, '')
    .replace(/\\9/g, '')
}
    \`

    console.log('[Theme Loader] Appending Styles')
  })()`
}

async function handleClientModThemeInjection() {
  const ts = document.createElement('style')
  ts.id = 'dorion-client-mods-themes'
  document.body.appendChild(ts)

  updateOverlay({
    midtitle: 'Loading client mod theme CSS...'
  })

  // Get the initial theme
  const themeContents = await invoke<string>('load_mods_css')

  // This will use the DOM in a funky way to validate the css, then we make sure to fix up quotes
  const cleanContents = cssSanitize(themeContents)?.replaceAll('\\"', '\'')

  return `;(() => {
    const ts = document.querySelector('#dorion-client-mods-themes')
    ts.textContent = \`
      ${cleanContents?.replace(/`/g, '\\`')
  // To this day I do not know why I need to do this
    .replace(/\\8/g, '')
    .replace(/\\9/g, '')
}
    \`

    console.log('[Theme Loader] Appending Client Mod Styles')
  })()`
}

/**
 * Display the splashscreen
 */
async function displayLoadingTop() {
  const html = await invoke<string>('get_index')
  const loadingContainer = document.createElement('div') satisfies HTMLDivElement
  loadingContainer.id = 'loadingContainer'
  loadingContainer.innerHTML = html

  loadingContainer.style.zIndex = '99999'
  loadingContainer.style.position = 'absolute'
  loadingContainer.style.top = '0'
  loadingContainer.style.left = '0'
  loadingContainer.style.width = '100vw'
  loadingContainer.style.height = '100vh'

  document.body.appendChild(loadingContainer)
}
