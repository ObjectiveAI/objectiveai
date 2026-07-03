/**
 * Typed run listener — the JS mirror of the Rust SDK's
 * `cli::websocket_listener::WebSocketListener`, fed by the viewer's
 * inbound plugin-event bus instead of a raw WebSocket.
 *
 * The daemon broadcasts every CLI run as one request frame
 * (`{…context, id, value: <Request>}`), then bare `{id, value: <item>}`
 * response frames (no type tag — the id is the whole routing story),
 * then exactly one terminator (`{id, end: true}`). The host routes a
 * plugin's frames into its iframe; [`RunListener`] turns them into
 * typed [`Run`] envelopes `{request, agentArguments, response}`,
 * discriminated purely off the request's path: unary leaves pair the
 * request with a `Promise`, streaming leaves with a [`RunStream`] —
 * mirroring the generated execute functions' return types. The
 * type-level union and the `path_type → mode` table are generated into
 * `./runEnvelope` by `scripts/install-run-envelope.cjs`.
 *
 * No validation: the types are structural claims over the wire; frame
 * bodies pass through as-is (in-band `CliError` lines arrive as items
 * of the same union the execute functions yield). A run whose
 * `path_type` this build predates — or a transform-bearing request
 * that doesn't parse — is SKIPPED entirely: no envelope, its frames
 * dropped. Transform-bearing requests (jq/python set) land on
 * `TransformedRun` with raw JSON items.
 */

import { listen } from "./index";
import { RUN_ENVELOPE_MAP, type Run } from "./runEnvelope";
import { RunStream } from "./pluginRuns";
import { type CliCommandAgentArguments } from "../cli/command/agentArguments";

/** A run's live feed-side handles, keyed by broadcast id. */
type LiveFeed = {
  pathType: string;
  stream?: RunStream<unknown>;
  resolve?: (value: unknown) => void;
  settled?: boolean;
};

/** Envelope retention cap — oldest runs drop off first. */
const RETAINED_RUNS_MAX = 256;

/**
 * The singleton run listener. Constructing it always returns the one
 * shared instance (lazily saved on first construction — including its
 * `subType`, which defaults to `"plugins_run"`, the sub_type the host
 * bridge routes a plugin's daemon frames under). The listener IS a
 * stream of [`Run`] envelopes:
 *
 * ```ts
 * for await (const run of new RunListener()) {
 *   if (run.request.path_type === "plugins/run") {
 *     for await (const item of run.response) { … }
 *   }
 * }
 * ```
 *
 * Envelopes are retained (capped) from construction onward; every
 * iterator replays the retained envelopes, then follows live. The
 * nested `response` never rides the root stream — it lives inside
 * each envelope, as a `Promise` or a [`RunStream`].
 */
export class RunListener {
  static #instance: RunListener | undefined;

  #retained: Run[] = [];
  #live = new Map<string, LiveFeed>();
  #skipped = new Set<string>();
  #subscribers = new Set<(run: Run) => void>();
  #unlisten: () => void = () => {};

  constructor(options?: { subType?: string }) {
    if (RunListener.#instance) {
      // The saved singleton — constructing again hands it back.
      return RunListener.#instance;
    }
    RunListener.#instance = this;
    this.#unlisten = listen<unknown>(
      options?.subType ?? "plugins_run",
      (frame) => {
        this.#onFrame(frame);
      },
    );
  }

  /**
   * Iterate the runs: retained envelopes first (up to
   * {@link RETAINED_RUNS_MAX}), then live as they arrive. Multiple
   * iterators are independent; `return()`/`break` detaches cleanly.
   */
  runs(): AsyncIterableIterator<Run> {
    const queue: Run[] = [...this.#retained];
    let wake: (() => void) | null = null;
    const push = (run: Run) => {
      queue.push(run);
      wake?.();
    };
    this.#subscribers.add(push);
    const detach = () => this.#subscribers.delete(push);
    return {
      next: async (): Promise<IteratorResult<Run>> => {
        while (queue.length === 0) {
          await new Promise<void>((resolve) => {
            wake = resolve;
          });
          wake = null;
        }
        return { value: queue.shift() as Run, done: false };
      },
      return: async (): Promise<IteratorResult<Run>> => {
        detach();
        return { value: undefined, done: true };
      },
      throw: async (e?: unknown): Promise<IteratorResult<Run>> => {
        detach();
        throw e;
      },
      [Symbol.asyncIterator]() {
        return this;
      },
    };
  }

  [Symbol.asyncIterator](): AsyncIterableIterator<Run> {
    return this.runs();
  }

  /** Reset the singleton (tests only). */
  static __resetForTests(): void {
    const instance = RunListener.#instance;
    if (instance) {
      instance.#unlisten();
    }
    RunListener.#instance = undefined;
  }

  #onFrame(frame: unknown): void {
    if (typeof frame !== "object" || frame === null) return;
    const f = frame as { id?: unknown; end?: unknown; value?: unknown };
    if (typeof f.id !== "string") return;

    // Frames carry no type tag — the id is the whole routing story:
    // terminator by `end: true`, response when the id is live (or
    // skipped), request otherwise.
    if (f.end === true) {
      this.#onEnd(f.id);
    } else if (this.#live.has(f.id)) {
      this.#onResponse(f.id, f.value);
    } else if (this.#skipped.has(f.id)) {
      // A run we chose not to open — drop its frames.
    } else {
      this.#onRequest(f.id, frame as Record<string, unknown>);
    }
  }

  #onRequest(id: string, frame: Record<string, unknown>): void {
    const request = frame["value"];
    if (typeof request !== "object" || request === null) {
      this.#skipped.add(id);
      return;
    }
    const req = request as {
      path_type?: unknown;
      jq?: unknown;
      python?: unknown;
      dangerous_advanced?: { stream?: unknown } | null;
    };
    // A request always carries its `path_type`; tag-less frames that
    // reach here without one are not requests.
    if (typeof req.path_type !== "string") {
      this.#skipped.add(id);
      return;
    }
    const agentArguments = extractAgentArguments(frame);

    // Mode off the request alone — the same rules as the Rust
    // dispatch: transform-bearing requests are raw-item streams; the
    // dual-form leaves pick per-request off the stream flag; an
    // unmapped path skips the run.
    const transformed = req.jq != null || req.python != null;
    const spec = RUN_ENVELOPE_MAP[req.path_type];
    if (spec === undefined && !transformed) {
      this.#skipped.add(id);
      return;
    }
    const mode = transformed
      ? "stream"
      : spec.mode === "both"
        ? req.dangerous_advanced?.stream === true
          ? "stream"
          : "unary"
        : spec.mode;

    let response: RunStream<unknown> | Promise<unknown>;
    const feed: LiveFeed = { pathType: req.path_type };
    if (mode === "stream") {
      feed.stream = new RunStream<unknown>();
      response = feed.stream;
    } else {
      response = new Promise<unknown>((resolve) => {
        feed.resolve = resolve;
      });
    }
    this.#live.set(id, feed);

    // The runtime can't prove the request↔response pairing the
    // generated union declares — the mode lookup IS that pairing.
    const envelope = { request, agentArguments, response } as Run;
    this.#retained.push(envelope);
    if (this.#retained.length > RETAINED_RUNS_MAX) {
      this.#retained.shift();
    }
    for (const subscriber of [...this.#subscribers]) {
      subscriber(envelope);
    }
  }

  #onResponse(id: string, value: unknown): void {
    const feed = this.#live.get(id);
    if (!feed) return;
    if (feed.stream) {
      feed.stream._push(value);
    } else if (!feed.settled) {
      feed.settled = true;
      feed.resolve?.(value);
    }
  }

  #onEnd(id: string): void {
    this.#skipped.delete(id);
    const feed = this.#live.get(id);
    if (!feed) return;
    this.#live.delete(id);
    if (feed.stream) {
      feed.stream._end();
    } else if (!feed.settled) {
      // A unary run that ended without any response frame settles with
      // a synthesized in-band error — the same union member the
      // execute functions yield.
      feed.settled = true;
      feed.resolve?.({
        type: "error",
        level: "error",
        fatal: null,
        message: `${feed.pathType}: run ended before any response item`,
      });
    }
  }
}

/** Pick the agent-argument fields off a request frame's context.
 * `mcp_session_id` is never teed onto the broadcast; the frame's
 * `plugin_*` coordinates are not agent arguments and are dropped. */
function extractAgentArguments(
  frame: Record<string, unknown>,
): CliCommandAgentArguments {
  const agentArguments: CliCommandAgentArguments = {};
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
  return agentArguments;
}
