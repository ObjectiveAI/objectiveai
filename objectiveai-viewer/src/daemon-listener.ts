/**
 * The app's ONE daemon broadcast connection — a fully autonomous
 * global singleton. No streams are exposed anywhere: consumers
 * REGISTER typed callbacks with [`registerExecutionHandler`], and the
 * singleton pumps in the background, firing every matching root
 * handler on every announced execution and driving the returned item
 * callbacks over that execution's response stream. Feature logic
 * lives in the handlers (mutating their own module state); React
 * reads that state through per-caller hooks.
 *
 * Dispatch guarantees:
 * - Root handlers run SYNCHRONOUSLY on execution receipt, so item
 *   callbacks attach before any response item can arrive — no
 *   subscribe-late races.
 * - Each execution's response is drained AT MOST ONCE — one shared
 *   iterator, items fanned to every attached callback in order —
 *   and not at all when no handler attached.
 * - Per-callback try/catch everywhere: one throwing handler never
 *   disturbs the others or the pump.
 * - Registrations live above the connection, spanning reconnects
 *   (connect-iterate-reconnect, 1s pause, same address). A daemon
 *   restart binds a fresh port, so recovery across restarts means
 *   restarting the viewer via `objectiveai viewer spawn`.
 *
 * `startDaemonListener()` is idempotent (React StrictMode safe) and
 * registers the built-in plugins/run forwarding itself: every
 * `plugins/run` execution is re-packaged into the standard
 * three-frame envelope (host-minted id per execution) and delivered
 * into the target plugin's iframe via the bridge's `deliverInbound`,
 * feeding the SDK's `ViewerPluginListener`. Outside Tauri (browser
 * dev) there is no daemon config; the singleton warns once and stays
 * idle.
 */
import {
  WebSocketListener,
  type ResponseItemStream,
  type CliCommandListenerExecution,
} from "@objectiveai/sdk";
import { tauriInvoke } from "./lib/tauri";
import { deliverInbound } from "./plugin-bridge";

// ── Typed registration surface ──────────────────────────────────────

/** Every request path the daemon can announce. */
export type PathType = CliCommandListenerExecution["request"]["path_type"];

/** The envelope for one path (dual-form paths yield their Variant
 * union — narrow on the response shape or the request's
 * `dangerous_advanced.stream`). */
export type ExecutionFor<P extends PathType> = Extract<
  CliCommandListenerExecution,
  { request: { path_type: P } }
>;

/** One path's response item union: the streaming item type, or the
 * unary resolved type for Promise responses. */
export type ItemFor<P extends PathType> =
  ExecutionFor<P>["response"] extends infer R
    ? R extends ResponseItemStream<infer T>
      ? T
      : R extends Promise<infer T>
        ? T
        : never
    : never;

/** Item-level hooks for one execution's response. */
export interface ExecutionItemHandler<P extends PathType = PathType> {
  /** Called with each response item, in order. */
  onItem?: (item: ItemFor<P>) => void;
  /** Called exactly once when the execution's response ends (the
   * terminator; in-band error items don't end streams). */
  onEnd?: () => void;
}

/** Root-level handler for one or more paths: fired on every announced
 * execution of those paths. Return item hooks (a bare function
 * normalizes to `{onItem}`) to observe the execution's response, or
 * nothing to ignore it. */
export type ExecutionHandler<P extends PathType> = (
  execution: ExecutionFor<P>,
) => ExecutionItemHandler<P> | ((item: ItemFor<P>) => void) | void;

/** Internal, type-erased registry entry. */
type AnyHandler = (
  execution: CliCommandListenerExecution,
) => ExecutionItemHandler | ((item: unknown) => void) | void;

const handlers = new Map<PathType, Set<AnyHandler>>();
let started = false;

/**
 * Register a typed handler for `paths`. Returns an unregister
 * function. Register everything at viewer startup — the singleton is
 * live-only, and executions announced before a registration existed
 * are gone for it.
 */
export function registerExecutionHandler<P extends PathType>(
  paths: P | readonly P[],
  handler: ExecutionHandler<P>,
): () => void {
  const list = (Array.isArray(paths) ? paths : [paths]) as readonly PathType[];
  const erased = handler as AnyHandler;
  for (const path of list) {
    let set = handlers.get(path);
    if (!set) {
      set = new Set();
      handlers.set(path, set);
    }
    set.add(erased);
  }
  return () => {
    for (const path of list) {
      handlers.get(path)?.delete(erased);
    }
  };
}

/** Start the singleton (idempotent). Call once at React startup,
 * after registering the app's handlers. */
export function startDaemonListener(): void {
  if (started) return;
  started = true;
  registerPluginsForwarding();
  void pumpForever();
}

// ── The pump ────────────────────────────────────────────────────────

/** The connect-iterate-reconnect loop. */
async function pumpForever(): Promise<void> {
  const config = await tauriInvoke<{
    address: string;
    signature: string | null;
  }>("websocket_config");
  if (!config) {
    // eslint-disable-next-line no-console
    console.warn(
      "daemon-listener: websocket_config unavailable (not running under " +
        "Tauri); no daemon executions will arrive.",
    );
    return;
  }
  for (;;) {
    try {
      const listener = await WebSocketListener.connect(
        `${config.address}/listen`,
        { signature: config.signature },
      );
      for await (const execution of listener) {
        dispatch(execution);
      }
    } catch {
      // Connect refused / handshake failure — fall through to retry.
    }
    // Connection over: brief pause, then a fresh attempt against the
    // same address.
    await new Promise((r) => setTimeout(r, 1000));
  }
}

/** One announced execution: fire the path's root handlers
 * synchronously, then drive the attached item hooks over one shared
 * drain of the response. */
function dispatch(execution: CliCommandListenerExecution): void {
  const set = handlers.get(execution.request.path_type);
  if (!set || set.size === 0) return;

  const attached: ExecutionItemHandler[] = [];
  for (const handler of [...set]) {
    try {
      const hooks = handler(execution);
      if (typeof hooks === "function") {
        attached.push({ onItem: hooks });
      } else if (hooks) {
        attached.push(hooks);
      }
    } catch (e) {
      // eslint-disable-next-line no-console
      console.error("daemon-listener: execution handler threw:", e);
    }
  }
  if (attached.length > 0) {
    void drain(execution.response, attached);
  }
}

/** Drain one execution's response exactly once, fanning each item to
 * every attached hook in order, then firing the end hooks. Unary
 * (Promise) responses normalize to a single item. */
async function drain(
  response: unknown,
  attached: ExecutionItemHandler[],
): Promise<void> {
  const emit = (item: unknown) => {
    for (const hooks of attached) {
      try {
        hooks.onItem?.(item as never);
      } catch (e) {
        // eslint-disable-next-line no-console
        console.error("daemon-listener: item callback threw:", e);
      }
    }
  };
  try {
    if (
      response !== null &&
      typeof response === "object" &&
      Symbol.asyncIterator in response
    ) {
      for await (const item of response as AsyncIterable<unknown>) {
        emit(item);
      }
    } else {
      emit(await (response as Promise<unknown>));
    }
  } finally {
    for (const hooks of attached) {
      try {
        hooks.onEnd?.();
      } catch (e) {
        // eslint-disable-next-line no-console
        console.error("daemon-listener: end callback threw:", e);
      }
    }
  }
}

// ── Built-in handler: plugins/run forwarding ────────────────────────

/** Re-package every plugins/run execution into the standard
 * three-frame envelope and deliver it into the target plugin's
 * iframe — the registry's first customer. Best-effort: an
 * unregistered/closed tab drops silently. */
function registerPluginsForwarding(): void {
  registerExecutionHandler("plugins/run", (execution) => {
    const coords = {
      owner: execution.request.owner,
      name: execution.request.name,
      version: execution.request.version,
    };
    const id = crypto.randomUUID();
    // Request frame: the agent arguments' fields flattened alongside
    // `id` and the request — the daemon's own wrapper shape.
    deliverInbound(coords, {
      ...execution.agentArguments,
      id,
      value: execution.request,
    });
    return {
      onItem: (item) => deliverInbound(coords, { id, value: item }),
      onEnd: () => deliverInbound(coords, { id, end: true }),
    };
  });
}

/** Reset the singleton (tests only). */
export function __resetForTests(): void {
  started = false;
  handlers.clear();
}
