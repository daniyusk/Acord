import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

type Equal<Left, Right> =
  (<Value>() => Value extends Left ? 1 : 2) extends
  (<Value>() => Value extends Right ? 1 : 2)
    ? true
    : false

type Expect<Value extends true> = Value
type IsAny<Value> = 0 extends (1 & Value) ? true : false

type InvokeResult = Awaited<ReturnType<typeof invoke>>
type ListenResult = ReturnType<typeof listen>

export type InvokeDoesNotReturnAny = Expect<Equal<IsAny<InvokeResult>, false>>
export type ListenReturnsAsyncUnlisten = Expect<
  Equal<ListenResult, Promise<UnlistenFn>>
>
