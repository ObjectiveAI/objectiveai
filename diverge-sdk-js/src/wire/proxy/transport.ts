// What the app supplies: its `invoke` and its channels.

/**
 * A structural mirror of a Tauri `Channel<T>`: the object MUST be a
 * real one — its serialization is what wires the stream across the
 * boundary — and this package touches only `onmessage`.
 */
export interface TransportChannel<T> {
  onmessage: (message: T) => void;
}

/**
 * The two things a Tauri app hands this package, taken structurally
 * so that `@tauri-apps/api` is never imported here: its `invoke`,
 * and a factory for its `Channel`. The app writes, once:
 *
 *     wire.proxy.handle({ invoke: (c, a) => invoke(c, a), channel: () => new Channel() })
 */
export interface Transport {
  /** Tauri's `invoke`, as the app imports it. */
  invoke(command: string, args: Record<string, unknown>): Promise<unknown>;
  /** A fresh Tauri `Channel`. */
  channel<T>(): TransportChannel<T>;
}
