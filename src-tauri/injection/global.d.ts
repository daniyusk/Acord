export {}

/* eslint-disable @typescript-eslint/no-explicit-any */
type TauriGlobal = typeof import('@tauri-apps/api')
type TauriShell = typeof import('@tauri-apps/plugin-shell')

declare global {
  interface Window {
    SHELTER_INJECTOR_PLUGINS: Record<string, [string, Record<string, unknown>]> 
    __TAURI__: TauriGlobal & {
      shell: TauriShell
      http: any
    }

    nativeFetch: typeof fetch
    __DORION_CONFIG__: Record<string, any>
    __DORION_INIT__: boolean
    __DORION_REAL_INIT__: boolean
    __DORION_TITLEBAR_KEEPER__: boolean
    Dorion: any
    shelter: any
    nativeOpen: Window['open']

    // Defined in initialization_script
    __localStorage: Storage
  }
}
