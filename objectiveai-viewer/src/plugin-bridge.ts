/**
 * Host-side bridge between plugin iframes and the daemon.
 *
 * Plugins never see daemon credentials and never touch Tauri: their
 * ONLY channel is postMessage with this host window. The reverse
 * direction (iframe → host) carries `cli-execute` (a typed
 * `cli::command::Request` as serde JSON — there is no raw-argv path)
 * plus the invocation `id` the iframe minted. This module catches it,
 * resolves the originating iframe via `MessageEvent.source` (a plugin
 * never claims an identity itself; unregistered windows are dropped),
 * runs the request through the host's own JS-native
 * [`WebSocketExecutor`] against the daemon's `/execute` route — the
 * daemon address + auth signature come from the Rust `daemon_config`
 * Tauri command, fetched once — and posts every response line back
 * into THAT iframe as
 * `{kind: "plugin-event", type: "cli_command", id, value}`, finishing
 * with the synthetic `{"type":"end"}` marker the SDK's
 * `ViewerPluginExecutor` terminates on. The `id` rides every line, so
 * concurrent invocations from one plugin demux cleanly.
 *
 * Failures (daemon config unavailable, connect refused) surface as
 * one in-band `{"type":"error",…}` line, then the end marker — the
 * iframe's iterator always terminates.
 *
 * Host → iframe `inbound` data delivery (e.g. `plugins/run` runs)
 * does not exist yet; it returns with the deferred plugins/run
 * routing, JS-native like everything else here.
 */
import { WebSocketExecutor } from "@objectiveai/sdk";
import { tauriInvoke } from "./lib/tauri";

/** A plugin's full install coordinates — the identity of a tab. */
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

/** Register a plugin iframe so the bridge can serve its executions. */
export function registerIframe(
  coords: PluginCoords,
  iframe: HTMLIFrameElement,
  src: string,
): void {
  const targetOrigin = deriveTargetOrigin(src);
  iframes.set(coordsKey(coords), { coords, iframe, targetOrigin });
  ensureReverseListener();
}

/** Unregister a previously-registered iframe. */
export function unregisterIframe(coords: PluginCoords): void {
  iframes.delete(coordsKey(coords));
}

// ── The host's daemon executor (lazy, shared) ─────────────────────

let executorPromise: Promise<WebSocketExecutor> | null = null;

/** The daemon config handed over by the Rust side, fetched once. */
async function daemonExecutor(): Promise<WebSocketExecutor> {
  if (!executorPromise) {
    executorPromise = (async () => {
      const config = await tauriInvoke<{
        address: string;
        signature: string | null;
      }>("daemon_config");
      if (!config) {
        throw new Error("daemon_config unavailable (not running under Tauri)");
      }
      return new WebSocketExecutor(`${config.address}/execute`, {
        signature: config.signature,
        agentArguments: { agent_instance_hierarchy: "Viewer" },
      });
    })();
    // A failed fetch shouldn't poison every later invocation.
    executorPromise.catch(() => {
      executorPromise = null;
    });
  }
  return executorPromise;
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
    // The invocation id is REQUIRED: it's what lets the iframe demux
    // concurrent runs.
    if (msg.request === undefined || msg.request === null) return;
    if (typeof msg.id !== "string" || msg.id.length === 0) return;
    const handle = findPluginByWindow(event.source);
    if (!handle) return;
    void runForIframe(handle, msg.id, msg.request);
  }
}

/** Run one plugin-requested execution against the daemon and post
 * every line — then the synthetic end marker — back into the
 * originating iframe. Delivery best-effort: if the iframe vanished
 * mid-run (tab closed), posts go nowhere and the run is dropped. */
async function runForIframe(
  handle: IframeHandle,
  id: string,
  request: unknown,
): Promise<void> {
  const post = (value: unknown) => {
    handle.iframe.contentWindow?.postMessage(
      { kind: "plugin-event", type: "cli_command", id, value },
      handle.targetOrigin,
    );
  };
  try {
    const executor = await daemonExecutor();
    for await (const line of executor.execute(
      request as Parameters<WebSocketExecutor["execute"]>[0],
    )) {
      post(line);
    }
  } catch (e) {
    post({ type: "error", level: "error", fatal: null, message: String(e) });
  }
  // The host-synthesized terminator the SDK's ViewerPluginExecutor
  // ends on.
  post({ type: "end" });
}

function findPluginByWindow(
  source: MessageEventSource | null,
): IframeHandle | null {
  if (!source) return null;
  for (const handle of iframes.values()) {
    if (handle.iframe.contentWindow === source) return handle;
  }
  return null;
}
