export * from "./apiCallBridge";
export * from "./generatedIndex";

/**
 * @objectiveai/sdk/viewer
 *
 * TypeScript shim that lets a plugin's iframe-mounted UI bundle
 * subscribe to events from the host viewer AND invoke the
 * objectiveai CLI in-process on the host.
 *
 * Production context (loaded inside the host viewer):
 *   - The plugin's `index.html` runs inside an `<iframe sandbox>`
 *     pointed at `plugin://localhost/<repo>/`.
 *   - `window.parent` is the host viewer's React app.
 *   - `listen(sub_type, handler)` registers a callback for incoming
 *     `inbound` events forwarded by the host bridge.
 *   - `invokeCli(args)` posts a `cli-invoke` message to the host;
 *     the host spawns the objectiveai cli binary and streams each
 *     output line back as a `cli_command` event. The returned
 *     AsyncIterable yields each line and terminates on the host's
 *     synthetic `{"type":"end"}` marker.
 *
 * Dev context (plugin author runs their own Tauri shell standalone):
 *   - `window.parent === window` (no host).
 *   - `listen()` falls through to `@tauri-apps/api`'s `listen`.
 *   - `invokeCli()` warns and yields nothing.
 *
 * The plugin author writes the same code in both contexts.
 */

/** True when running inside an iframe in the host viewer. */
function isInIframe(): boolean {
  return typeof window !== "undefined" && window.parent !== window;
}

interface InboundMessage {
  kind: "plugin-event";
  type: "inbound";
  sub_type: string;
  value: unknown;
}

interface CliCommandMessage {
  kind: "plugin-event";
  type: "cli_command";
  value: unknown;
}

type PluginEventMessage = InboundMessage | CliCommandMessage;

const inboundListeners = new Map<string, Set<(value: unknown) => void>>();
let inboundHandlerAttached = false;

function attachInboundHandler(): void {
  if (inboundHandlerAttached) return;
  if (typeof window === "undefined") return;
  window.addEventListener("message", (event: MessageEvent) => {
    const msg = event.data as PluginEventMessage | null;
    if (!msg || typeof msg !== "object") return;
    if (msg.kind !== "plugin-event") return;
    if (msg.type !== "inbound") return;
    const set = inboundListeners.get(msg.sub_type);
    if (!set) return;
    for (const fn of set) {
      try {
        fn(msg.value);
      } catch (e) {
        // Don't let one handler take down the others.
        // eslint-disable-next-line no-console
        console.error("@objectiveai/sdk/viewer listener threw:", e);
      }
    }
  });
  inboundHandlerAttached = true;
}

/**
 * Register a handler for incoming `inbound` plugin events. Returns
 * an unsubscribe function.
 *
 * `sub_type` matches the `sub_type` field of the `Event::Inbound`
 * emitted by the Rust host — the string the plugin author
 * registered in their manifest's `viewer_routes` entry, or one of
 * the built-in event names (e.g. `agent_completions`).
 *
 * In iframe context the events come from the host's bridge; in
 * standalone-dev context they come from `@tauri-apps/api`'s `listen`.
 */
export function listen<T = unknown>(
  sub_type: string,
  handler: (value: T) => void,
): () => void {
  if (!isInIframe()) {
    // Standalone dev mode: register via @tauri-apps/api.
    let unlisten: (() => void) | null = null;
    let cancelled = false;
    void (async () => {
      try {
        const mod = await import("@tauri-apps/api/event");
        if (cancelled) return;
        const u = await mod.listen<{ value: T }>(`plugin-${sub_type}`, (e) =>
          handler(e.payload?.value as T),
        );
        if (cancelled) {
          u();
        } else {
          unlisten = u;
        }
      } catch {
        // eslint-disable-next-line no-console
        console.warn(
          `@objectiveai/sdk/viewer: listen('${sub_type}') called outside an iframe and ` +
            `@tauri-apps/api is unavailable; events will not fire.`,
        );
      }
    })();
    return () => {
      cancelled = true;
      if (unlisten) unlisten();
    };
  }
  attachInboundHandler();
  const set = inboundListeners.get(sub_type) ?? new Set();
  const fn = (value: unknown) => handler(value as T);
  set.add(fn);
  inboundListeners.set(sub_type, set);
  return () => {
    const s = inboundListeners.get(sub_type);
    if (!s) return;
    s.delete(fn);
    if (s.size === 0) inboundListeners.delete(sub_type);
  };
}

/**
 * Run the objectiveai cli binary on the host with `args` (the host
 * spawns it via the SDK's BinaryExecutor). Returns an
 * `AsyncIterable<unknown>` over the JSONL output lines the cli
 * emits. Each yielded value is the parsed cli output envelope:
 * either `{"type":"error",…}` or
 * `{"type":"notification","value":{"kind":"<variant>",…}}`. The
 * inner `kind` discriminator selects a `NotificationValue` variant
 * (see the schemas under `cli.output.notification.*`). The host
 * appends a synthetic `{"type":"end"}` line when the cli's stdout
 * closes; the iterator terminates on it.
 *
 * The plugin author never specifies `destination` — the host bridge
 * derives it from the iframe's identity. To run multiple cli
 * invocations sequentially, `await` each iterator to completion
 * before starting the next; concurrent invocations from the same
 * iframe produce interleaved streams that this shim cannot demux
 * (no per-invocation sub_type on `cli_command` events).
 *
 * Requires the objectiveai cli to be installed on the host
 * (`~/.objectiveai/objectiveai`). When the spawn fails the host
 * streams back a `{"type":"error",…}` envelope followed by
 * `{"type":"end"}`, so the iterator still terminates.
 */
export function invokeCli(args: string[]): AsyncIterable<unknown> {
  return {
    [Symbol.asyncIterator]() {
      const queue: unknown[] = [];
      let resolveNext: (() => void) | null = null;
      let done = false;
      let cleaned = false;

      const onMessage = (event: MessageEvent) => {
        const msg = event.data as CliCommandMessage | null;
        if (!msg || typeof msg !== "object") return;
        if (msg.kind !== "plugin-event" || msg.type !== "cli_command") return;
        queue.push(msg.value);
        // End-of-stream signal: the HOST synthesizes {"type":"end"}
        // when the spawned cli's stdout closes — the cli itself never
        // emits it.
        if (
          typeof msg.value === "object" &&
          msg.value !== null &&
          (msg.value as { type?: string }).type === "end"
        ) {
          done = true;
        }
        if (resolveNext) {
          const r = resolveNext;
          resolveNext = null;
          r();
        }
      };

      const cleanup = () => {
        if (cleaned) return;
        cleaned = true;
        if (typeof window !== "undefined") {
          window.removeEventListener("message", onMessage);
        }
      };

      if (typeof window === "undefined" || window.parent === window) {
        // Standalone dev mode: no host to invoke the cli. Warn and
        // produce an empty iterator.
        // eslint-disable-next-line no-console
        console.warn(
          "@objectiveai/sdk/viewer: invokeCli() called outside an iframe; " +
            "no host present, the iterator will yield nothing.",
        );
        done = true;
      } else {
        window.addEventListener("message", onMessage);
        // Fire the request. The bridge derives `destination` from
        // event.source — caller never sets it.
        window.parent.postMessage({ kind: "cli-invoke", args }, "*");
      }

      return {
        async next(): Promise<IteratorResult<unknown>> {
          while (queue.length === 0 && !done) {
            await new Promise<void>((r) => {
              resolveNext = r;
            });
          }
          if (queue.length > 0) {
            return { value: queue.shift(), done: false };
          }
          cleanup();
          return { value: undefined, done: true };
        },
        async return(): Promise<IteratorResult<unknown>> {
          cleanup();
          return { value: undefined, done: true };
        },
      };
    },
  };
}

/** Internal-use: clear in-flight state. Exposed for tests only.
 * Note: the module-level `message` event listener stays attached —
 * removing/re-attaching it would just register a duplicate. The
 * listeners map is what carries per-test state. */
export function __resetForTests(): void {
  inboundListeners.clear();
}
