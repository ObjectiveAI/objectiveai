/**
 * Host-side bridge between plugin iframes and Tauri.
 *
 * The Rust side emits every event on one of two Tauri channels, keyed
 * by the event's `destination`:
 *
 *   - `"objectiveai"` — events for the main viewer UI (this bridge
 *     never touches them);
 *   - `"plugin"` — events for plugin iframes; the payload's
 *     `destination.plugin` carries the plugin's FULL coordinates
 *     (`owner`/`name`/`version`).
 *
 * This bridge does no routing beyond delivery: it subscribes to the
 * `"plugin"` channel once and posts each event into the matching
 * registered iframe as a `{kind: "plugin-event", type, value}`
 * postMessage. Two event types flow host → iframe today:
 *
 *   - `cli_command` — one response line from a viewer-executor
 *     invocation THIS plugin started, terminated by a synthetic
 *     `{"type":"end"}` line (whoever runs a request gets its own
 *     response back);
 *   - `inbound` — reserved; nothing is emitted to plugin destinations
 *     on it yet (`plugins/run` delivery comes later).
 *
 * The reverse direction (iframe → host) carries `cli-execute` (a
 * typed `cli::command::Request` as serde JSON — there is no raw-argv
 * path) plus the invocation `id` the iframe minted. This module
 * catches it, resolves the originating iframe via
 * `MessageEvent.source`, and dispatches the `cli_execute` Tauri
 * command with the plugin's full coordinates as the `destination` —
 * a plugin never claims an identity itself. The `id` rides every
 * response event back, so concurrent invocations from one plugin
 * demux cleanly. Messages from unknown windows (or without an id)
 * are dropped.
 */
import { tauriListen as safeListen, tauriInvoke } from "./lib/tauri";

/** A plugin's full install coordinates — the identity events route by. */
export type PluginCoords = {
  owner: string;
  name: string;
  version: string;
};

type IframeHandle = {
  coords: PluginCoords;
  iframe: HTMLIFrameElement;
  /**
   * Target origin for host -> iframe postMessage calls. Derived once
   * at register time — always `"*"` under the current sandbox; see
   * `deriveTargetOrigin` for why nothing stricter can deliver.
   */
  targetOrigin: string;
};

type PluginDestination = { plugin: PluginCoords };

type EventPayload = {
  type: "inbound" | "cli_command";
  destination: PluginDestination | "objectiveai";
  /** cli_command only: the caller-minted invocation id — rides every
   * response line so concurrent invocations demux in the iframe. */
  id?: string;
  value: unknown;
};

const iframes = new Map<string, IframeHandle>();

function coordsKey(coords: PluginCoords): string {
  return `${coords.owner}/${coords.name}/${coords.version}`;
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

/** Register a plugin iframe so the bridge can deliver its events. */
export function registerIframe(
  coords: PluginCoords,
  iframe: HTMLIFrameElement,
  src: string,
): void {
  const targetOrigin = deriveTargetOrigin(src);
  iframes.set(coordsKey(coords), { coords, iframe, targetOrigin });
  void ensurePluginListener();
  ensureReverseListener();
}

/** Unregister a previously-registered iframe. */
export function unregisterIframe(coords: PluginCoords): void {
  iframes.delete(coordsKey(coords));
}

// ── Host → iframe: the shared "plugin" channel ─────────────────────

let pluginListenerStarted = false;

async function ensurePluginListener(): Promise<void> {
  if (pluginListenerStarted) return;
  pluginListenerStarted = true;
  await safeListen<EventPayload>("plugin", (event) => {
    const payload = event.payload;
    if (!payload || typeof payload !== "object") return;
    if (payload.type !== "inbound" && payload.type !== "cli_command") return;
    const destination = payload.destination;
    if (
      !destination ||
      typeof destination !== "object" ||
      !("plugin" in destination)
    ) {
      return;
    }
    const handle = iframes.get(coordsKey(destination.plugin));
    if (!handle) return;
    handle.iframe.contentWindow?.postMessage(
      payload.type === "cli_command"
        ? {
            kind: "plugin-event",
            type: payload.type,
            id: payload.id,
            value: payload.value,
          }
        : { kind: "plugin-event", type: payload.type, value: payload.value },
      handle.targetOrigin,
    );
  });
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
        id?: unknown;
        request?: unknown;
      }
    | null;
  if (!msg || typeof msg !== "object") return;

  if (msg.kind === "cli-execute") {
    // Typed request: serde JSON of the SDK's `cli::command::Request`.
    // The Rust side runs it through the daemon's /execute route, then
    // streams cli_command events back to this plugin's destination —
    // fire-and-forget. There is deliberately no raw-argv invocation
    // path. The invocation id is REQUIRED: it's what lets the iframe
    // demux concurrent runs.
    if (msg.request === undefined || msg.request === null) return;
    if (typeof msg.id !== "string" || msg.id.length === 0) return;
    const coords = findPluginByWindow(event.source);
    if (!coords) return;
    void tauriInvoke("cli_execute", {
      request: msg.request,
      id: msg.id,
      destination: { plugin: coords },
    });
  }
}

function findPluginByWindow(
  source: MessageEventSource | null,
): PluginCoords | null {
  if (!source) return null;
  for (const handle of iframes.values()) {
    if (handle.iframe.contentWindow === source) return handle.coords;
  }
  return null;
}
