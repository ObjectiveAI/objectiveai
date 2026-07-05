/**
 * The app's ONE daemon broadcast connection — a singleton wrapper
 * around the SDK's `WebSocketListener` that handles all routing and
 * forwarding:
 *
 * - **Routing (in-app):** every run fans out to [`daemonRuns`]
 *   subscribers — the feed the UI consumes (live-only,
 *   multi-subscriber, survives reconnects).
 * - **Forwarding (plugins):** every `plugins/run` run is re-packaged
 *   into the standard three-frame envelope and delivered into the
 *   target plugin's iframe via the bridge's `deliverInbound` —
 *   feeding the SDK's `ViewerPluginListener`. The frame id is
 *   host-minted per run (`crypto.randomUUID()`), the same policy the
 *   old Rust passthrough used: consumers only need per-run
 *   consistency.
 *
 * Lifecycle: `startDaemonListener()` is idempotent (React StrictMode
 * safe) and runs for the app's life — the daemon config comes from
 * the Rust `daemon_config` command once; each `WebSocketListener` is
 * one connection, and when it ends (daemon drop) the wrapper sleeps
 * 1s and reconnects to the same address (a daemon restart binds a
 * fresh port, so recovery across restarts means restarting the
 * viewer via `objectiveai viewer spawn` — same policy as ever).
 * Outside Tauri (browser dev) there is no daemon config; the wrapper
 * warns once and stays idle.
 */
import {
  WebSocketListener,
  type CliCommandListenerExecution,
  type CliCommandPluginsRunListenerExecution,
} from "@objectiveai/sdk";
import { tauriInvoke } from "./lib/tauri";
import { deliverInbound } from "./plugin-bridge";

/** One live subscriber: a per-iterator pending queue's feed side. */
type Subscriber = {
  push: (run: CliCommandListenerExecution) => void;
};

let started = false;
const subscribers = new Set<Subscriber>();

/** Start the singleton (idempotent). Call once at React startup. */
export function startDaemonListener(): void {
  if (started) return;
  started = true;
  void pumpForever();
}

/**
 * Iterate every run the daemon announces, from this call onward —
 * live-only, independent per iterator, `break` detaches cleanly.
 * Never ends (the wrapper reconnects underneath); the UI consumes
 * this across connection generations without noticing.
 */
export function daemonRuns(): AsyncIterableIterator<CliCommandListenerExecution> {
  const queue: CliCommandListenerExecution[] = [];
  let ended = false;
  let wake: (() => void) | null = null;
  const subscriber: Subscriber = {
    push: (run) => {
      queue.push(run);
      wake?.();
    },
  };
  subscribers.add(subscriber);
  // Detaching also ENDS the iterator: a consumer parked in `next()`
  // when someone else calls `return()` (e.g. an unmount cleanup while
  // the consumer loop is awaiting) wakes up and sees `done` instead
  // of hanging on a wake that would never come.
  const detach = () => {
    subscribers.delete(subscriber);
    ended = true;
    wake?.();
  };
  return {
    next: async (): Promise<IteratorResult<CliCommandListenerExecution>> => {
      for (;;) {
        if (queue.length > 0) {
          return {
            value: queue.shift() as CliCommandListenerExecution,
            done: false,
          };
        }
        if (ended) {
          return { value: undefined, done: true };
        }
        await new Promise<void>((resolve) => {
          wake = resolve;
        });
        wake = null;
      }
    },
    return: async (): Promise<IteratorResult<CliCommandListenerExecution>> => {
      detach();
      return { value: undefined, done: true };
    },
    throw: async (
      e?: unknown,
    ): Promise<IteratorResult<CliCommandListenerExecution>> => {
      detach();
      throw e;
    },
    [Symbol.asyncIterator]() {
      return this;
    },
  };
}

/** The connect-iterate-reconnect loop. */
async function pumpForever(): Promise<void> {
  const config = await tauriInvoke<{
    address: string;
    signature: string | null;
  }>("daemon_config");
  if (!config) {
    // eslint-disable-next-line no-console
    console.warn(
      "daemon-listener: daemon_config unavailable (not running under " +
        "Tauri); no daemon runs will arrive.",
    );
    return;
  }
  for (;;) {
    try {
      const listener = await WebSocketListener.connect(
        `${config.address}/listen`,
        { signature: config.signature },
      );
      for await (const run of listener) {
        onRun(run);
      }
    } catch {
      // Connect refused / handshake failure — fall through to retry.
    }
    // Connection over: brief pause, then a fresh attempt against the
    // same address.
    await new Promise((r) => setTimeout(r, 1000));
  }
}

/** One announced run: fan out to the app, forward plugins/run to its
 * plugin. Both attach to the live-only response stream synchronously,
 * before yielding back to the event loop, so no item can be missed. */
function onRun(run: CliCommandListenerExecution): void {
  for (const subscriber of [...subscribers]) {
    subscriber.push(run);
  }
  if (run.request.path_type === "plugins/run") {
    void forwardRun(run as CliCommandPluginsRunListenerExecution);
  }
}

/** Re-package one plugins/run run into the standard three-frame
 * envelope and deliver it into the target plugin's iframe — the JS
 * successor of the old Rust passthrough's `emit_run`. Best-effort:
 * an unregistered/closed tab drops silently. */
async function forwardRun(
  run: CliCommandPluginsRunListenerExecution,
): Promise<void> {
  const coords = {
    owner: run.request.owner,
    name: run.request.name,
    version: run.request.version,
  };
  const id = crypto.randomUUID();

  // Request frame: the agent arguments' fields flattened alongside
  // `id` and the request — the daemon's own wrapper shape.
  deliverInbound(coords, { ...run.agentArguments, id, value: run.request });
  for await (const item of run.response) {
    deliverInbound(coords, { id, value: item });
  }
  deliverInbound(coords, { id, end: true });
}

/** Reset the singleton (tests only). */
export function __resetForTests(): void {
  started = false;
  subscribers.clear();
}
