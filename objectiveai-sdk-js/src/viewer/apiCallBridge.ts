/**
 * JS-side bridge for the `viewer: true` mode of the `ObjectiveAI`
 * client.
 *
 * When `client.viewer === true`, every HTTP method on the client
 * routes through this module instead of `fetch()`. We post an
 * `api-call-invoke` message to the host plugin-bridge, which
 * dispatches to the `api_call_run` Tauri command. The host streams
 * one or more `api_call` events back; we collect them in an
 * AsyncIterable and either:
 *
 *   - return the single `Chunk` (unary path), or
 *   - re-emit each `Chunk` through a `Stream<T>` (streaming path).
 *
 * Wire format of each event's `value` is the `ApiCallEnvelope`
 * tagged enum:
 *
 *   - `{type: "begin"}`           — emitted once, ignored here
 *   - `{type: "chunk", chunk: T}` — one per SSE event (streaming) or
 *                                   one total (unary)
 *   - `{type: "error", error: o}` — terminates the stream by throwing
 *   - `{type: "end"}`             — terminates the iterator cleanly
 *
 * Demux: `sub_type` on each incoming event matches the `subType`
 * (METHOD_PATH string) of the original invocation. Concurrent calls
 * to *different* endpoints from the same iframe are correctly
 * separated; concurrent calls to the *same* endpoint will interleave
 * (no per-invocation correlation id — same limitation as
 * `invokeCli`).
 */
import { ObjectiveAIFetchError } from "../error";
import { Stream } from "../stream";
import type { ViewerApiCallEnvelope } from "./apiCallEnvelope";
import type { ViewerApiCallSubType } from "./apiCallSubType";
import type { ViewerHttpMethod } from "./httpMethod";

/** True when running inside an iframe in the host viewer. */
function isInIframe(): boolean {
  return typeof window !== "undefined" && window.parent !== window;
}

/**
 * `{kind: "plugin-event", type: "api_call", ...}` postMessage shape
 * that the host's plugin-bridge forwards to iframes. The `sub_type`
 * and `value` fields are typed against the auto-generated zod
 * schemas for `viewer.ApiCallSubType` and `viewer.ApiCallEnvelope`.
 */
type ApiCallMessage = {
  kind: "plugin-event";
  type: "api_call";
  sub_type: ViewerApiCallSubType;
  value: ViewerApiCallEnvelope;
};

/**
 * Build the `<METHOD>_<PATH>` sub_type string that matches the Rust
 * [`ApiCallSubType`] serde rename. The resulting string is
 * type-asserted against the auto-generated `ViewerApiCallSubType`
 * union (35 enum members) — invalid `(method, path)` combinations
 * will produce a non-matching string at runtime and the host will
 * fail to dispatch.
 */
function buildSubType(method: ViewerHttpMethod, path: string): ViewerApiCallSubType {
  return `${method}_${path}` as ViewerApiCallSubType;
}

/**
 * Start an api-call-invoke session against the host. Returns an
 * AsyncIterable of `chunk` values (the SSE event bodies or, for
 * unary endpoints, the single response body). The iterator terminates
 * on the `end` envelope and throws an [`ObjectiveAIFetchError`] on
 * `error` envelopes.
 *
 * Bails synchronously if not running in an iframe (no host present).
 */
export function viewerApiCallChunks<T>(
  method: ViewerHttpMethod,
  path: string,
  body: unknown,
): AsyncIterable<T> {
  if (!isInIframe()) {
    throw new Error(
      `ObjectiveAI({ viewer: true }) used outside a viewer iframe; ` +
        `cannot route ${method} ${path} through Tauri. Construct the ` +
        `client with viewer: false (or omit it) when running outside ` +
        `the viewer host.`,
    );
  }
  const subType = buildSubType(method, path);

  return {
    [Symbol.asyncIterator]() {
      const queue: T[] = [];
      let resolveNext: (() => void) | null = null;
      let done = false;
      let errored: ObjectiveAIFetchError | Error | null = null;
      let cleaned = false;

      const onMessage = (event: MessageEvent) => {
        const msg = event.data as ApiCallMessage | null;
        if (!msg || typeof msg !== "object") return;
        if (msg.kind !== "plugin-event") return;
        if (msg.type !== "api_call") return;
        if (msg.sub_type !== subType) return;

        const env: ViewerApiCallEnvelope | undefined = msg.value;
        if (!env || typeof env !== "object") return;

        if (env.type === "begin") {
          // No-op marker.
        } else if (env.type === "chunk") {
          queue.push(env.chunk as T);
        } else if (env.type === "error") {
          // Wrap as ObjectiveAIFetchError if the inner shape looks
          // like a ResponseError, else surface the raw error message.
          const raw = env.error as { message?: unknown } | null;
          const message = raw && typeof raw === "object" && typeof raw.message === "string"
            ? raw.message
            : JSON.stringify(env.error);
          errored = new ObjectiveAIFetchError(0, message);
          done = true;
        } else if (env.type === "end") {
          done = true;
        }

        if (resolveNext) {
          const r = resolveNext;
          resolveNext = null;
          r();
        }
      };

      const cleanup = () => {
        if (cleaned) return;
        cleaned = true;
        if (typeof window !== "undefined") {
          window.removeEventListener("message", onMessage);
        }
      };

      window.addEventListener("message", onMessage);
      // Fire the request. The host derives `origin` from
      // event.source (the iframe's contentWindow), so the caller
      // never sets it.
      window.parent.postMessage(
        { kind: "api-call-invoke", subType, body: body ?? null },
        "*",
      );

      return {
        async next(): Promise<IteratorResult<T>> {
          while (queue.length === 0 && !done) {
            await new Promise<void>((r) => {
              resolveNext = r;
            });
          }
          if (queue.length > 0) {
            return { value: queue.shift() as T, done: false };
          }
          cleanup();
          if (errored) throw errored;
          return { value: undefined, done: true };
        },
        async return(): Promise<IteratorResult<T>> {
          cleanup();
          return { value: undefined, done: true };
        },
      };
    },
  };
}

/**
 * Make a viewer-mode unary API call. Awaits the first `chunk`
 * envelope and returns it; subsequent chunks are silently discarded
 * (unary endpoints emit exactly one). Throws on `error` envelopes or
 * if no chunk arrives before `end`.
 */
export async function viewerApiCallUnary<T>(
  method: ViewerHttpMethod,
  path: string,
  body: unknown,
): Promise<T> {
  const iter = viewerApiCallChunks<T>(method, path, body);
  for await (const chunk of iter) {
    return chunk;
  }
  throw new Error(
    `viewer api call ${method} ${path} produced no chunks before end`,
  );
}

/**
 * Make a viewer-mode unary API call with no expected response body.
 * Drains the iterator without inspecting any chunk values; throws on
 * `error` envelopes.
 */
export async function viewerApiCallUnaryNoResponse(
  method: ViewerHttpMethod,
  path: string,
  body: unknown,
): Promise<void> {
  const iter = viewerApiCallChunks<unknown>(method, path, body);
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  for await (const _ of iter) {
    // Drain.
  }
}

/**
 * Make a viewer-mode streaming API call. Returns a `Stream<T>`
 * backed by the chunk iterator so consumer code that does
 * `for await (const chunk of stream)` works identically to fetch
 * mode.
 */
export function viewerApiCallStream<T>(
  method: ViewerHttpMethod,
  path: string,
  body: unknown,
): Stream<T> {
  return Stream.fromAsyncIterable(viewerApiCallChunks<T>(method, path, body));
}
