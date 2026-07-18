export {}

/* eslint-disable @typescript-eslint/no-explicit-any */
declare global {
  interface Window {
    SHELTER_INJECTOR_PLUGINS: Record<string, [string, Record<string, unknown>]> 
    __TAURI__: {
      shell: {
        open: (path: string) => void
      }
      http: any
      [key: string]: unknown
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
