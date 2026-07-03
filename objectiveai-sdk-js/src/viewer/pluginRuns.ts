/**
 * Typed receiver for the daemon-broadcast runs routed into a plugin's
 * iframe — the read side matching the viewer executor's write side.
 *
 * The host bridge forwards a plugin's `plugins/run` daemon frames into
 * its iframe as `plugins_run` plugin-events. Each run arrives as one
 * request frame (`{…context, id, value: <Request>}`), then response
 * frames (`{id, path_type, value: <item>}`), then exactly one
 * terminator (`{id, path_type, end: true}`). [`PluginRunListener`]
 * turns that wire into typed [`PluginRun`] envelopes
 * (`{request, context, response}`), where `response` mirrors the
 * return type of the request's generated execute function — a
 * `Promise` for unary leaves, a [`RunStream`] for streaming leaves —
 * discriminated entirely off the request (its `path_type`, plus the
 * `dangerous_advanced.stream` flag on multi-variant leaves). The
 * type-level union and the runtime schema map are generated into
 * `./runEnvelope` by `scripts/install-run-envelope.cjs`.
 *
 * Transform caveat: a run whose request carries `jq`/`python` teed
 * post-transform JSON — its items intentionally skip leaf validation
 * and pass through raw (the envelope still types by `path_type`; the
 * daemon applies the same validate-or-passthrough leniency).
 */

import { z } from "zod";
import { listen } from "./index";
import { RUN_ENVELOPE_MAP, type PluginRun } from "./runEnvelope";

/**
 * The producer context stamped on a run's request frame — the agent
 * identity / plugin coordinates of whoever ran the command — plus the
 * broadcast stream `id`.
 */
export type RunContext = {
  id: string;
  agent_instance_hierarchy?: string | null;
  agent_id?: string | null;
  agent_full_id?: string | null;
  agent_remote?: string | null;
  response_id?: string | null;
  response_ids?: string | null;
  plugin_owner?: string | null;
  plugin_repository?: string | null;
  plugin_version?: string | null;
};

/** One `path_type`'s runtime knowledge in the generated map. */
export type RunEnvelopeSpec = {
  /**
   * `stream` / `unary`, or `both` for multi-variant leaves (a unary
   * primary plus a streaming twin gated on
   * `dangerous_advanced.stream: true` — resolved per request).
   */
  mode: "stream" | "unary" | "both";
  /** Streaming item validator: `z.union([CliError, <Item>])`. */
  itemSchema?: z.ZodType<unknown>;
  /** Unary response validator: `z.union([CliError, <Response>])`. */
  responseSchema?: z.ZodType<unknown>;
};

/**
 * Replay-buffer async-iterable for one run's streaming response.
 * Items are retained for the run's lifetime (runs are bounded by the
 * terminator frame), so any number of consumers can iterate — each
 * replays the buffer, then follows live until the terminator ends the
 * stream.
 */
export class RunStream<T> implements AsyncIterable<T> {
  #items: T[] = [];
  #done = false;
  #waiters: Array<() => void> = [];

  /** Items received so far (live view; do not mutate). */
  get items(): readonly T[] {
    return this.#items;
  }

  /** Whether the run's terminator has arrived. */
  get done(): boolean {
    return this.#done;
  }

  /** @internal — feed side. */
  _push(item: T): void {
    if (this.#done) return;
    this.#items.push(item);
    this.#wake();
  }

  /** @internal — feed side. */
  _end(): void {
    this.#done = true;
    this.#wake();
  }

  #wake(): void {
    const waiters = this.#waiters;
    this.#waiters = [];
    for (const wake of waiters) wake();
  }

  async *[Symbol.asyncIterator](): AsyncIterator<T> {
    let i = 0;
    for (;;) {
      while (i < this.#items.length) {
        yield this.#items[i++];
      }
      if (this.#done) return;
      await new Promise<void>((resolve) => this.#waiters.push(resolve));
    }
  }

  /** Every item, resolved once the run's terminator arrives. */
  async toArray(): Promise<T[]> {
    const out: T[] = [];
    for await (const item of this) out.push(item);
    return out;
  }
}

/** A run's live feed-side handles, keyed by broadcast id. */
type LiveRun = {
  pathType: string;
  stream?: RunStream<unknown>;
  resolve?: (value: unknown) => void;
  reject?: (reason: Error) => void;
  settled?: boolean;
  schema?: z.ZodType<unknown>;
};

/** Envelope retention cap — oldest runs drop off first. */
const RETAINED_RUNS_MAX = 256;

/**
 * The plugin's singleton run listener. Constructing it always returns
 * the one shared instance (lazily saved on first construction), so
 * every corner of a plugin's UI can `new PluginRunListener()` and
 * observe the same run history:
 *
 * ```ts
 * for await (const run of new PluginRunListener()) {
 *   if (run.request.path_type === "plugins/run") {
 *     for await (const item of run.response) { … }
 *   }
 * }
 * ```
 *
 * Envelopes are retained (capped) from construction onward; every
 * iterator replays the retained envelopes, then follows live.
 */
export class PluginRunListener {
  static #instance: PluginRunListener | undefined;

  #retained: PluginRun[] = [];
  #live = new Map<string, LiveRun>();
  #subscribers = new Set<(run: PluginRun) => void>();
  #unlisten: () => void;

  constructor() {
    if (PluginRunListener.#instance) {
      // The saved singleton — constructing again hands it back.
      return PluginRunListener.#instance;
    }
    PluginRunListener.#instance = this;
    this.#unlisten = listen<unknown>("plugins_run", (frame) => {
      this.#onFrame(frame);
    });
  }

  /**
   * Iterate the plugin's runs: retained envelopes first (up to
   * {@link RETAINED_RUNS_MAX}), then live as they arrive. Multiple
   * iterators are independent; `return()`/`break` detaches cleanly.
   */
  runs(): AsyncIterableIterator<PluginRun> {
    const queue: PluginRun[] = [...this.#retained];
    let wake: (() => void) | null = null;
    const push = (run: PluginRun) => {
      queue.push(run);
      wake?.();
    };
    this.#subscribers.add(push);
    const detach = () => this.#subscribers.delete(push);
    return {
      next: async (): Promise<IteratorResult<PluginRun>> => {
        while (queue.length === 0) {
          await new Promise<void>((resolve) => {
            wake = resolve;
          });
          wake = null;
        }
        return { value: queue.shift() as PluginRun, done: false };
      },
      return: async (): Promise<IteratorResult<PluginRun>> => {
        detach();
        return { value: undefined, done: true };
      },
      throw: async (e?: unknown): Promise<IteratorResult<PluginRun>> => {
        detach();
        throw e;
      },
      [Symbol.asyncIterator]() {
        return this;
      },
    };
  }

  [Symbol.asyncIterator](): AsyncIterableIterator<PluginRun> {
    return this.runs();
  }

  /** Reset the singleton (tests only). */
  static __resetForTests(): void {
    PluginRunListener.#instance?.#unlisten?.();
    PluginRunListener.#instance = undefined;
  }

  #onFrame(frame: unknown): void {
    if (typeof frame !== "object" || frame === null) return;
    const f = frame as {
      id?: unknown;
      path_type?: unknown;
      end?: unknown;
      value?: unknown;
    };
    if (typeof f.id !== "string") return;

    if (f.path_type === undefined) {
      this.#onRequestFrame(f.id, frame as Record<string, unknown>);
    } else if (f.end === true) {
      this.#onEndFrame(f.id);
    } else {
      this.#onResponseFrame(f.id, f.value);
    }
  }

  #onRequestFrame(id: string, frame: Record<string, unknown>): void {
    const request = frame["value"];
    if (typeof request !== "object" || request === null) return;
    const req = request as {
      path_type?: unknown;
      jq?: unknown;
      python?: unknown;
      dangerous_advanced?: { stream?: unknown } | null;
    };
    const pathType = typeof req.path_type === "string" ? req.path_type : "";
    const context = extractContext(id, frame);

    // Resolve the run's mode + validator off the request. Unknown
    // path_type → raw stream fallback; a transform request (jq/python
    // set) keeps its shape but skips leaf validation (its items are
    // post-transform JSON).
    const spec = RUN_ENVELOPE_MAP[pathType];
    const transformed = req.jq != null || req.python != null;
    const mode =
      spec === undefined
        ? "stream"
        : spec.mode === "both"
          ? req.dangerous_advanced?.stream === true
            ? "stream"
            : "unary"
          : spec.mode;
    const schema = transformed
      ? undefined
      : mode === "stream"
        ? spec?.itemSchema
        : spec?.responseSchema;

    let response: RunStream<unknown> | Promise<unknown>;
    const live: LiveRun = { pathType, schema };
    if (mode === "stream") {
      live.stream = new RunStream<unknown>();
      response = live.stream;
    } else {
      const promise = new Promise<unknown>((resolve, reject) => {
        live.resolve = resolve;
        live.reject = reject;
      });
      // Keep an un-awaited unary response from surfacing as an
      // unhandled rejection; consumers awaiting still see it.
      promise.catch(() => {});
      response = promise;
    }
    this.#live.set(id, live);

    // The runtime can't prove the request↔response pairing the
    // generated union declares — the map lookup IS that pairing.
    const envelope = { request, context, response } as PluginRun;
    this.#retained.push(envelope);
    if (this.#retained.length > RETAINED_RUNS_MAX) {
      this.#retained.shift();
    }
    for (const subscriber of [...this.#subscribers]) {
      subscriber(envelope);
    }
  }

  #onResponseFrame(id: string, value: unknown): void {
    const live = this.#live.get(id);
    if (!live) return;
    // Validate when the leaf is known; pass raw through otherwise —
    // the same validate-or-passthrough leniency the daemon applies.
    let item = value;
    if (live.schema) {
      const parsed = live.schema.safeParse(value);
      if (parsed.success) item = parsed.data;
    }
    if (live.stream) {
      live.stream._push(item);
    } else if (!live.settled) {
      live.settled = true;
      live.resolve?.(item);
    }
  }

  #onEndFrame(id: string): void {
    const live = this.#live.get(id);
    if (!live) return;
    this.#live.delete(id);
    if (live.stream) {
      live.stream._end();
    } else if (!live.settled) {
      live.settled = true;
      live.reject?.(
        new Error(`${live.pathType}: run ended before any response item`),
      );
    }
  }
}

/** Pick the known context fields off a request frame. */
function extractContext(id: string, frame: Record<string, unknown>): RunContext {
  const context: RunContext = { id };
  for (const key of [
    "agent_instance_hierarchy",
    "agent_id",
    "agent_full_id",
    "agent_remote",
    "response_id",
    "response_ids",
    "plugin_owner",
    "plugin_repository",
    "plugin_version",
  ] as const) {
    const value = frame[key];
    if (typeof value === "string" || value === null) {
      context[key] = value;
    }
  }
  return context;
}
