/**
 * Viewer-mode transport for the daemon streams: instead of a direct
 * fetch+SSE connection ({@link connectSse}), the stream rides a Tauri
 * IPC channel driven by the viewer's Rust process — one Tauri command
 * per daemon endpoint, each forwarding the endpoint's raw SSE `data`
 * payloads verbatim. The webview holds NO daemon connections (its
 * per-origin HTTP connection budget starved the direct-fetch model),
 * no address, no auth signature, and no agent identity — the Rust
 * proxy owns all three.
 *
 * The SDK never imports Tauri: the viewer injects a
 * {@link ViewerTransport} built from `@tauri-apps/api/core`
 * (`invoke` + `Channel`), typed here structurally.
 */

/** Structural mirror of a `@tauri-apps/api/core` `Channel<T>` — the
 * object MUST be a real Tauri `Channel` instance (its serialization
 * is what wires the stream); the SDK only touches `onmessage`. */
export interface ViewerTransportChannel<T> {
  onmessage: (message: T) => void;
}

/** The injected transport: Tauri `invoke` plus a `Channel` factory.
 * Built in the viewer app from `@tauri-apps/api/core`. */
export interface ViewerTransport {
  invoke(cmd: string, args: Record<string, unknown>): Promise<unknown>;
  channel<T>(): ViewerTransportChannel<T>;
}

/** One per-stream channel event from the Rust proxy (serde-tagged):
 * `data` carries one SSE event's raw `data` payload verbatim, `end`
 * is the daemon closing the stream normally (the fetch-mode body
 * end), `error` is a mid-stream transport failure. */
export type ViewerStreamEvent =
  | { type: "data"; data: string }
  | { type: "end" }
  | { type: "error"; message: string };

/** A fresh per-stream id for `daemon_stream_close` correlation. */
export function newStreamId(): string {
  return (
    globalThis.crypto?.randomUUID?.() ??
    Math.random().toString(36).slice(2) + Date.now().toString(36)
  );
}

/**
 * Viewer-mode mirror of {@link connectSse}: invoke the proxy
 * `command` with a fresh stream id + channel, resolve on connect
 * success (the invoke promise — the Rust side resolves it once the
 * daemon's 2xx response headers arrive), reject on connect failure.
 * The returned generator yields each raw `data` payload, ends on
 * `end`, and throws on `error` — a drop-in for `connectSse`'s
 * generator, so callers keep their own parse + fold.
 *
 * `signal` is the ONE teardown path (parity with fetch abort):
 * aborting terminates the generator locally and fires
 * `daemon_stream_close`, which cancels the Rust-side connection (and
 * with it the daemon-side run). Abandoning the generator without
 * aborting leaks the stream, exactly like fetch mode. Reconnection
 * remains the caller's loop.
 */
export async function connectViewerStream(
  transport: ViewerTransport,
  command: string,
  args: Record<string, unknown>,
  signal: AbortSignal,
): Promise<AsyncGenerator<string>> {
  const streamId = newStreamId();
  // The event queue and its wake-up: `onmessage` is attached BEFORE
  // the invoke, because channel messages and the invoke promise's
  // settlement are separate IPC messages with no ordering guarantee —
  // events that race the connect resolution buffer here.
  const queue: ViewerStreamEvent[] = [];
  let wake: (() => void) | null = null;
  const push = (event: ViewerStreamEvent) => {
    queue.push(event);
    if (wake !== null) {
      const w = wake;
      wake = null;
      w();
    }
  };
  const channel = transport.channel<ViewerStreamEvent>();
  channel.onmessage = push;

  if (signal.aborted) {
    throw new Error(`connect daemon stream: ${command}: aborted`);
  }
  signal.addEventListener("abort", () => {
    // Local termination first (a pending read settles immediately),
    // then the Rust-side cancel — fire-and-forget, and a no-op there
    // when the stream already ended naturally.
    push({ type: "error", message: "aborted" });
    void transport
      .invoke("daemon_stream_close", { streamId })
      .catch(() => {});
  });

  try {
    await transport.invoke(command, { ...args, streamId, onEvent: channel });
  } catch (e) {
    throw new Error(`connect daemon stream: ${command}: ${String(e)}`);
  }

  return (async function* () {
    for (;;) {
      while (queue.length > 0) {
        const event = queue.shift()!;
        switch (event.type) {
          case "data":
            yield event.data;
            break;
          case "end":
            return;
          case "error":
            throw new Error(event.message);
        }
      }
      await new Promise<void>((resolve) => {
        wake = resolve;
      });
    }
  })();
}
