/**
 * Host-side bridge between plugin iframes and Tauri.
 *
 * For each registered plugin iframe, this module subscribes to the
 * matching `<repository>` Tauri channel and forwards each emitted
 * `Event` into the iframe as a `{kind: 'plugin-event', ...}`
 * postMessage. The iframe consumes those via `@objectiveai/sdk/viewer`'s
 * `listen()` (or by adding its own `window.addEventListener('message')`).
 *
 * Three event variants flow host -> iframe:
 *
 *   - `inbound` — host has data for the plugin (the existing path).
 *     The iframe's `listen(sub_type, handler)` matches on the
 *     `sub_type` discriminator.
 *
 *   - `cli_command` — one line of stdout from an in-process
 *     `objectiveai_cli::run()` invocation that this iframe started
 *     via `invokeCli`. No sub_type.
 *
 *   - `api_call` — one envelope (begin / chunk / error / end) from a
 *     viewer-mode `ObjectiveAI` client call that this iframe started
 *     via `api-call-invoke`. The `sub_type` is the
 *     `"<METHOD>_<PATH>"` rename of the targeted endpoint, so the
 *     iframe can demux concurrent calls to *different* endpoints.
 *
 * The reverse direction (iframe -> host) carries `cli-invoke` and
 * `api-call-invoke` postMessages; this module catches them, resolves
 * the originating iframe via `MessageEvent.source`, and dispatches
 * the matching Tauri command (`cli_run` / `api_call_run`) with the
 * originator's plugin name as `origin`. Messages from unknown
 * sources are dropped (security: don't let a random iframe drive
 * the host without identity).
 */
import { invoke } from "@tauri-apps/api/core";
import { listen as tauriListen, type UnlistenFn } from "@tauri-apps/api/event";

type IframeHandle = {
  pluginName: string;
  iframe: HTMLIFrameElement;
};

type InboundPayload = {
  type: "inbound";
  destination: string;
  sub_type: string;
  value: unknown;
};

type CliCommandPayload = {
  type: "cli_command";
  destination: string;
  value: unknown;
};

type ApiCallPayload = {
  type: "api_call";
  destination: string;
  sub_type: string;
  value: unknown;
};

type EventPayload = InboundPayload | CliCommandPayload | ApiCallPayload;

const iframes = new Map<string, IframeHandle>();
const tauriUnlisteners = new Map<string, UnlistenFn>();

/** Register a plugin iframe so the bridge can forward events to it. */
export function registerIframe(pluginName: string, iframe: HTMLIFrameElement): void {
  iframes.set(pluginName, { pluginName, iframe });
  void subscribeToPluginEvents(pluginName);
  ensureReverseListener();
}

/** Unregister a previously-registered iframe. Cancels its event sub. */
export function unregisterIframe(pluginName: string): void {
  iframes.delete(pluginName);
  const unlisten = tauriUnlisteners.get(pluginName);
  if (unlisten) {
    unlisten();
    tauriUnlisteners.delete(pluginName);
  }
}

async function subscribeToPluginEvents(pluginName: string): Promise<void> {
  if (tauriUnlisteners.has(pluginName)) return;
  const unlisten = await tauriListen<EventPayload>(pluginName, (event) => {
    const handle = iframes.get(pluginName);
    if (!handle) return;
    const payload = event.payload;
    if (!payload || !payload.type) return;
    if (payload.type === "inbound") {
      handle.iframe.contentWindow?.postMessage(
        {
          kind: "plugin-event",
          type: "inbound",
          sub_type: payload.sub_type,
          value: payload.value,
        },
        "*",
      );
    } else if (payload.type === "cli_command") {
      handle.iframe.contentWindow?.postMessage(
        { kind: "plugin-event", type: "cli_command", value: payload.value },
        "*",
      );
    } else if (payload.type === "api_call") {
      handle.iframe.contentWindow?.postMessage(
        {
          kind: "plugin-event",
          type: "api_call",
          sub_type: payload.sub_type,
          value: payload.value,
        },
        "*",
      );
    }
  });
  tauriUnlisteners.set(pluginName, unlisten);
}

// ── Reverse channel: iframe -> host postMessages ──────────────────

let reverseListenerAttached = false;

function ensureReverseListener(): void {
  if (reverseListenerAttached) return;
  if (typeof window === "undefined") return;
  window.addEventListener("message", onIframeMessage);
  reverseListenerAttached = true;
}

function onIframeMessage(event: MessageEvent): void {
  const msg = event.data as
    | { kind?: string; args?: unknown; subType?: unknown; body?: unknown }
    | null;
  if (!msg || typeof msg !== "object") return;

  if (msg.kind === "cli-invoke") {
    if (!Array.isArray(msg.args)) return;
    const origin = findPluginByWindow(event.source);
    if (!origin) return;
    const args = msg.args.filter((a): a is string => typeof a === "string");
    // Fire-and-forget. The host streams cli_command events back via
    // the events bus; the iframe consumes them through its async
    // iterator.
    void invoke("cli_run", { args, origin });
    return;
  }

  if (msg.kind === "api-call-invoke") {
    if (typeof msg.subType !== "string") return;
    const origin = findPluginByWindow(event.source);
    if (!origin) return;
    // Fire-and-forget. The host streams api_call envelope events back
    // via the events bus; the iframe consumes them through its async
    // iterator. `subType` is the serde-rename of the targeted endpoint
    // (e.g. "POST_/agent/completions"); the body is whatever the JS
    // SDK would have sent over fetch().
    void invoke("api_call_run", {
      subType: msg.subType,
      body: msg.body ?? null,
      origin,
    });
    return;
  }
}

function findPluginByWindow(source: MessageEventSource | null): string | null {
  if (!source) return null;
  for (const [name, handle] of iframes) {
    if (handle.iframe.contentWindow === source) return name;
  }
  return null;
}
