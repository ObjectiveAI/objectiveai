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
 *   - `cli_command` — one stdout JSONL line from an objectiveai cli
 *     binary the host spawned for an `invokeCliRequest` this iframe
 *     started, terminated by a synthetic `{"type":"end"}` line. No
 *     sub_type.
 *
 *   - `api_call` — one envelope (begin / chunk / error / end) from a
 *     viewer-mode `ObjectiveAI` client call that this iframe started
 *     via `api-call-invoke`. The `sub_type` is the
 *     `"<METHOD>_<PATH>"` rename of the targeted endpoint, so the
 *     iframe can demux concurrent calls to *different* endpoints.
 *
 * The reverse direction (iframe -> host) carries `cli-execute` (a
 * typed `cli::command::Request` as serde JSON — there is no raw-argv
 * path) and `api-call-invoke` postMessages; this module catches
 * them, resolves the originating iframe via `MessageEvent.source`,
 * and dispatches the matching Tauri command (`cli_execute` /
 * `api_call_run`) with the originator's plugin name as `origin`.
 * Messages from unknown sources are dropped (security: don't let a
 * random iframe drive the host without identity).
 */
import { invoke } from "@tauri-apps/api/core";
import { listen as tauriListen, type UnlistenFn } from "@tauri-apps/api/event";

type IframeHandle = {
  pluginName: string;
  iframe: HTMLIFrameElement;
  /**
   * Target origin for host -> iframe postMessage calls. Derived once
   * at register time — always `"*"` under the current sandbox; see
   * `deriveTargetOrigin` for why nothing stricter can deliver.
   */
  targetOrigin: string;
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
export function registerIframe(
  pluginName: string,
  iframe: HTMLIFrameElement,
  src: string,
): void {
  const targetOrigin = deriveTargetOrigin(src);
  iframes.set(pluginName, { pluginName, iframe, targetOrigin });
  void subscribeToPluginEvents(pluginName);
  ensureReverseListener();
}

/**
 * Target origin for host -> iframe postMessage calls: always `"*"`.
 *
 * `PluginPane.tsx` sandboxes every plugin iframe with
 * `allow-scripts allow-forms` and no `allow-same-origin`, which gives
 * the iframe an opaque origin (`null`). An opaque origin never matches
 * a concrete targetOrigin, so posting to `"plugin://localhost"` or
 * `"https://plugin.example.com"` is silently dropped by the browser —
 * `invokeCli()` responses never arrive and the iterator hangs
 * (issue #203). `"*"` is the only value the browser will deliver.
 *
 * This is not a broadcast: every message is posted directly to
 * `handle.iframe.contentWindow`, so the window reference itself is
 * the delivery constraint, and the reverse direction keeps its
 * `findPluginByWindow` identity gate. If `allow-same-origin` is ever
 * added to the sandbox, derived-origin tightening
 * (`new URL(src).origin`) can be restored here for http(s)
 * `viewer_url` plugins — which is why the `src` plumbing stays.
 */
function deriveTargetOrigin(_src: string): string {
  return "*";
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
        handle.targetOrigin,
      );
    } else if (payload.type === "cli_command") {
      handle.iframe.contentWindow?.postMessage(
        { kind: "plugin-event", type: "cli_command", value: payload.value },
        handle.targetOrigin,
      );
    } else if (payload.type === "api_call") {
      handle.iframe.contentWindow?.postMessage(
        {
          kind: "plugin-event",
          type: "api_call",
          sub_type: payload.sub_type,
          value: payload.value,
        },
        handle.targetOrigin,
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
    | {
        kind?: string;
        request?: unknown;
        subType?: unknown;
        body?: unknown;
      }
    | null;
  if (!msg || typeof msg !== "object") return;

  if (msg.kind === "cli-execute") {
    // Typed request: serde JSON of the SDK's `cli::command::Request`.
    // The Rust side deserializes it and lowers it to argv via
    // `into_command()`, then streams cli_command events back via the
    // events bus — fire-and-forget. There is deliberately no raw-argv
    // invocation path.
    if (msg.request === undefined || msg.request === null) return;
    const origin = findPluginByWindow(event.source);
    if (!origin) return;
    void invoke("cli_execute", { request: msg.request, origin });
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
