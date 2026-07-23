import { openUrl } from '@tauri-apps/plugin-opener'

export function proxyXHR() {
  const open = XMLHttpRequest.prototype.open

  XMLHttpRequest.prototype.open = function(...args: unknown[]) {
    const [_method, url] = args
    const rgx = /\/api\/v.*\/(science|track)/g

    if (String(url).match(rgx)) {
      console.log(`[XHR Blocker] Blocked URL: ${url}`)
      return
    }

    // @ts-expect-error this is fine
    open.apply(this, args)
  }
}

export function proxyAddEventListener() {
  const originalAdd = window.addEventListener
  const originalRemove = window.removeEventListener
  const beforeUnloadListeners = new WeakMap<EventListenerOrEventListenerObject, EventListener>()

  window.addEventListener = function(...args: Parameters<typeof window.addEventListener>) {
    const [type, listener] = args
    if (type === 'beforeunload' && listener) {
      const wrapper: EventListener = (...listenerArgs: Parameters<EventListener>) => {
        // @ts-expect-error this is fine
        const isTrustedOverwrite = listenerArgs[0]?.isTrustedOverwrite

        if (isTrustedOverwrite !== undefined) {
          const event = listenerArgs[0]
          listenerArgs[0] = new Proxy(event, {
            get(target, prop, receiver) {
              if (prop === 'isTrusted') return isTrustedOverwrite
              return Reflect.get(target, prop, receiver)
            }
          })
        }

        return (typeof listener === 'object' && listener !== null && 'handleEvent' in listener)
          ? listener.handleEvent(...listenerArgs)
          : (listener as EventListener)(...listenerArgs)
      }

      beforeUnloadListeners.set(listener, wrapper)
      args[1] = wrapper
    }

    return originalAdd.apply(this, args)
  }

  window.removeEventListener = function(...args: Parameters<typeof window.removeEventListener>) {
    const [type, listener] = args
    if (type === 'beforeunload' && listener) {
      const wrapper = beforeUnloadListeners.get(listener)
      if (wrapper) {
        args[1] = wrapper
        beforeUnloadListeners.delete(listener)
      }
    }

    return originalRemove.apply(this, args)
  }
}

const INTERNAL_DOMAINS = [
  'discord.com',
  'discordapp.com',
  'cdn.discordapp.com'
]

function isExternal(url: string) {
  try {
    if (url.startsWith('/')) return false

    const parsed = new URL(url)

    return !INTERNAL_DOMAINS.some(domain =>
      parsed.hostname === domain || parsed.hostname.endsWith(`.${domain}`)
    )
  } catch {
    return false
  }
}

function isOpenableExternalUrl(url: string) {
  try {
    const parsed = new URL(url)

    if (!['http:', 'https:', 'mailto:', 'tel:'].includes(parsed.protocol)) return false
    if (parsed.protocol === 'mailto:' || parsed.protocol === 'tel:') return parsed.pathname.length > 0

    const hostname = parsed.hostname.toLowerCase()
    return hostname.length > 0
      && !parsed.username
      && !parsed.password
      && hostname !== 'localhost'
      && !hostname.endsWith('.localhost')
  } catch {
    return false
  }
}

export function proxyOpen() {
  // Open external links with the scoped opener plugin.
  window.nativeOpen = window.open
  window.open = (url: string | undefined | URL, target?: string, features?: string) => {

    if (!url) {
      return window.nativeOpen(url, target, features)
    }

    const urlStr = url.toString()
    // If this needs to open externally, do so
    if (urlStr !== 'about:blank' && (target === '_blank' || !target) && isExternal(urlStr)) {
      console.log('[Proxy Open] External URL:', urlStr)

      if (!isOpenableExternalUrl(urlStr)) return null

      void openUrl(urlStr).catch(error => console.error('[Proxy Open] Failed to open URL:', error))
      return null
    }

    console.log('[Proxy Open] Internal URL:', urlStr)
    
    const win = window.nativeOpen(urlStr, target, features)

    // Otherwise, use the native open
    return win
  }
}

export function proxyNotification() {
  let permVal = 'granted'

  // @ts-expect-error shut up
  window.nativeNotification = window.Notification

  // @ts-expect-error shut up
  window.Notification = function(..._args) {
    // Stub this
  }

  window.Notification.requestPermission = async () => 'granted'
  // For checking if we have stubbed
  Object.defineProperty(window.Notification, '__IS_STUBBED__', {
    enumerable: true,
    value: true
  })

  Object.defineProperty(window.Notification, 'permission', {
    enumerable: true,
    get: () => permVal,
    set: (v) => {
      permVal = v
    }
  })
}
