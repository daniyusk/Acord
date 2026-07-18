import type { UnlistenFn } from '@tauri-apps/api/event'

type Equal<Left, Right> =
  (<Value>() => Value extends Left ? 1 : 2) extends
  (<Value>() => Value extends Right ? 1 : 2)
    ? true
    : false

type Expect<Value extends true> = Value
type IsAny<Value> = 0 extends (1 & Value) ? true : false

type GlobalTauri = Window['__TAURI__']
type InvokeResult = Awaited<ReturnType<GlobalTauri['core']['invoke']>>
type ListenResult = ReturnType<GlobalTauri['event']['listen']>
type HasLegacyHttpFacade = 'http' extends keyof GlobalTauri ? true : false

export type InvokeDoesNotReturnAny = Expect<Equal<IsAny<InvokeResult>, false>>
export type ListenReturnsAsyncUnlisten = Expect<
  Equal<ListenResult, Promise<UnlistenFn>>
>
export type LegacyHttpFacadeIsRemoved = Expect<Equal<HasLegacyHttpFacade, false>>
