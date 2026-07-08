/**
 * Typed `plugins/run` receiver for PLUGIN IFRAMES — the receiving
 * half of a viewer plugin's SDK surface (`ViewerPluginExecutor` is
 * the sending half).
 *
 * A plugin never sees the daemon broadcast; what reaches its iframe
 * is the subset the HOST routes to it. The contract: every
 * `plugins/run` run targeting this plugin arrives as
 * `{kind: "plugin-event", type: "inbound", value: <frame>}`
 * postMessages, where `<frame>` is one standard broadcast-envelope
 * frame — the request (`{…context, id, value: <plugins/run Request>}`),
 * then `{id, value: <item>}` per response item, then exactly one
 * terminator (`{id, end: true}`) — with a host-minted per-run `id`.
 * [`ViewerPluginListener`] parses that stream and IS an
 * async-iterable of [`ViewerPluginRun`]s:
 *
 * ```ts
 * const listener = new ViewerPluginListener();
 * for await (const run of listener) {
 *   console.log(run.id, run.request.args);
 *   for await (const item of run.response) {
 *     // CliCommandPluginsRunResponseItem (Mcp | Notification) or an
 *     // in-band CliError line
 *   }
 * }
 * ```
 *
 * FIXED-TYPE: unlike [`WebSocketListener`]'s full generated tree,
 * every run here is a `plugins/run` — the host only ever routes this
 * plugin's own runs. (A defensive skipped-id set silently drops any
 * other `path_type`, should the host ever send one.) `run.id` is
 * host-minted: stable across the run's frames and unique per run —
 * correlate concurrent runs by it.
 *
 * PLUGIN ONLY: constructing this in the main viewer window throws —
 * there is no host above it to deliver plugin events (the main
 * viewer consumes the full broadcast with `WebSocketListener`).
 *
 * Delivery is LIVE-ONLY, like every listener surface: an iterator
 * receives runs announced after it subscribes; nothing is retained
 * (a plugin that mounts mid-run misses that run). `close()` detaches
 * from the window, ends every open response stream and every root
 * iterator.
 */

import { type CliCommandAgentArguments } from "./command/agentArguments";
import { type CliCommandPluginsRunRequest } from "./command/plugins/run/request";
import { type CliCommandPluginsRunResponseItem } from "./command/plugins/run/responseItem";
import { type CliError } from "./error";
import { ResponseItemStream } from "./websocketListener";

/** One `plugins/run` run the host delivered to this plugin. */
export type ViewerPluginRun = {
  /** Host-minted per-run id — stable across the run's frames, unique
   * per run; correlate concurrent runs by it. */
  id: string;
  request: CliCommandPluginsRunRequest;
  agentArguments: CliCommandAgentArguments;
  response: ResponseItemStream<CliError | CliCommandPluginsRunResponseItem>;
};

/** One live subscriber: a per-iterator pending queue's feed side. */
type Subscriber = {
  push: (run: ViewerPluginRun) => void;
  end: () => void;
};

export class ViewerPluginListener {
  #closed = false;
  #live = new Map<
    string,
    ResponseItemStream<CliError | CliCommandPluginsRunResponseItem>
  >();
  #skipped = new Set<string>();
  #subscribers = new Set<Subscriber>();
  #onMessage: (event: MessageEvent) => void;

  constructor() {
    if (typeof window === "undefined" || window.parent === window) {
      throw new Error(
        "ViewerPluginListener is plugin-only: it receives the plugins/run " +
          "frames the host viewer delivers into this plugin's iframe, and " +
          "there is no host above the main viewer window. (In the main " +
          "viewer, use WebSocketListener for the full daemon broadcast.)",
      );
    }
    this.#onMessage = (event: MessageEvent) => {
      const msg = event.data as {
        kind?: unknown;
        type?: unknown;
        value?: unknown;
      } | null;
      if (!msg || typeof msg !== "object") return;
      if (msg.kind !== "plugin-event") return;
      if (msg.type !== "inbound") return;
      this.#onFrame(msg.value);
    };
    window.addEventListener("message", this.#onMessage);
  }

  /** Detach from the window: every open response stream ends and
   * every root iterator ends. */
  close(): void {
    if (this.#closed) return;
    this.#closed = true;
    window.removeEventListener("message", this.#onMessage);
    for (const stream of this.#live.values()) {
      stream._end();
    }
    this.#live.clear();
    this.#skipped.clear();
    const subscribers = [...this.#subscribers];
    this.#subscribers.clear();
    for (const subscriber of subscribers) {
      subscriber.end();
    }
  }

  /**
   * Iterate the runs delivered from this call onward. Multiple
   * iterators are independent; `return()`/`break` detaches cleanly;
   * every iterator ends on [`ViewerPluginListener.close`].
   */
  runs(): AsyncIterableIterator<ViewerPluginRun> {
    const queue: ViewerPluginRun[] = [];
    let ended = this.#closed;
    let wake: (() => void) | null = null;
    const subscriber: Subscriber = {
      push: (run) => {
        queue.push(run);
        wake?.();
      },
      end: () => {
        ended = true;
        wake?.();
      },
    };
    if (!ended) {
      this.#subscribers.add(subscriber);
    }
    const detach = () => this.#subscribers.delete(subscriber);
    return {
      next: async (): Promise<IteratorResult<ViewerPluginRun>> => {
        for (;;) {
          if (queue.length > 0) {
            return { value: queue.shift() as ViewerPluginRun, done: false };
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
      return: async (): Promise<IteratorResult<ViewerPluginRun>> => {
        detach();
        return { value: undefined, done: true };
      },
      throw: async (e?: unknown): Promise<IteratorResult<ViewerPluginRun>> => {
        detach();
        throw e;
      },
      [Symbol.asyncIterator]() {
        return this;
      },
    };
  }

  [Symbol.asyncIterator](): AsyncIterableIterator<ViewerPluginRun> {
    return this.runs();
  }

  #onFrame(frame: unknown): void {
    if (typeof frame !== "object" || frame === null) return;
    const f = frame as { id?: unknown; end?: unknown; value?: unknown };
    if (typeof f.id !== "string") return;

    // Frames carry no type tag — the id is the whole routing story:
    // terminator by `end: true`, response when the id is live (or
    // skipped), request otherwise.
    if (f.end === true) {
      this.#skipped.delete(f.id);
      const stream = this.#live.get(f.id);
      if (stream) {
        this.#live.delete(f.id);
        stream._end();
      }
    } else if (this.#live.has(f.id)) {
      this.#live
        .get(f.id)!
        ._push(f.value as CliError | CliCommandPluginsRunResponseItem);
    } else if (this.#skipped.has(f.id)) {
      // A run we chose not to open — drop its frames.
    } else {
      this.#onRequest(f.id, frame as Record<string, unknown>);
    }
  }

  #onRequest(id: string, frame: Record<string, unknown>): void {
    const request = frame["value"];
    // The host only routes this plugin's own plugins/run runs; drop
    // anything else defensively (and remember the id so its later
    // frames drop too).
    if (
      typeof request !== "object" ||
      request === null ||
      (request as { path_type?: unknown }).path_type !== "plugins/run"
    ) {
      this.#skipped.add(id);
      return;
    }

    const response = new ResponseItemStream<
      CliError | CliCommandPluginsRunResponseItem
    >();
    this.#live.set(id, response);
    const run: ViewerPluginRun = {
      id,
      request: request as CliCommandPluginsRunRequest,
      agentArguments: extractAgentArguments(frame),
      response,
    };
    for (const subscriber of [...this.#subscribers]) {
      subscriber.push(run);
    }
  }
}

/** Pick the agent-argument fields off a request frame's context.
 * `mcp_session_id` is never teed onto the broadcast; the frame's
 * `plugin_*` coordinates are not agent arguments and are dropped.
 * (Local twin of the module-private helper in websocketListener.ts.) */
function extractAgentArguments(
  frame: Record<string, unknown>,
): CliCommandAgentArguments {
  const agentArguments: Record<string, string | null> = {};
  for (const key of [
    "agent_instance_hierarchy",
    "agent_id",
    "agent_full_id",
    "agent_remote",
    "response_id",
    "response_ids",
  ] as const) {
    const value = frame[key];
    if (typeof value === "string" || value === null) {
      agentArguments[key] = value;
    }
  }
  return agentArguments as CliCommandAgentArguments;
}
