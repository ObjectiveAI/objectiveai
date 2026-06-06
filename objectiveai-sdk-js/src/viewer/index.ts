export * from "./generatedIndex";
export * from "./cliStream";
export * from "./invoke";
export * from "./command/index";

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
 *   - `invokeCliRequest(request)` posts a `cli-execute` message with
 *     a typed `cli::command::Request` (serde JSON) to the host; the
 *     host lowers it to argv via `into_command()` and spawns the
 *     objectiveai cli binary, streaming each output line back as a
 *     `cli_command` event. The returned AsyncIterable yields each
 *     line and terminates on the host's synthetic `{"type":"end"}`
 *     marker. There is deliberately no raw-argv entry point.
 *
 * Dev context (plugin author runs their own Tauri shell standalone):
 *   - `window.parent === window` (no host).
 *   - `listen()` falls through to `@tauri-apps/api`'s `listen`.
 *   - `invokeCliRequest()` warns and yields nothing.
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

/** Internal-use: clear in-flight state. Exposed for tests only.
 * Note: the module-level `message` event listener stays attached —
 * removing/re-attaching it would just register a duplicate. The
 * listeners map is what carries per-test state. */
export function __resetForTests(): void {
  inboundListeners.clear();
}
