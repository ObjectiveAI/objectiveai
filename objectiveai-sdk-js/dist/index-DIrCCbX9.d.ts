import { z } from 'zod';

type JsonValue = string | number | boolean | null | JsonValue[] | {
    [key: string]: JsonValue;
};

/**
 * A readable stream wrapper for Server-Sent Events (SSE).
 * Provides async iteration over parsed SSE data events.
 * Throws ObjectiveAIFetchError if the API returns an error in the stream.
 *
 * Two backing sources are supported:
 *
 * 1. `new Stream(response, controller?)` — wraps an SSE-emitting
 *    HTTP response; this is the path used by `fetch`-mode clients.
 *
 * 2. `Stream.fromAsyncIterable(iter, controller?)` — wraps a
 *    pre-parsed AsyncIterable of chunks; used by `viewer: true`
 *    clients so the consumer-facing `Stream<T>` type stays identical
 *    across both fetch and Tauri-postMessage backends.
 */
declare class Stream<T> implements AsyncIterable<T> {
    private reader;
    private decoder;
    private buffer;
    private done;
    private controller;
    private asyncIterableSource;
    constructor(response: Response, controller?: AbortController | null);
    /**
     * Wrap an `AsyncIterable<T>` (e.g. the viewer-mode api-call
     * iterator) as a `Stream<T>`. Skips the SSE parsing path entirely;
     * the iterator yields chunks pre-parsed by its source.
     */
    static fromAsyncIterable<T>(source: AsyncIterable<T>, controller?: AbortController | null): Stream<T>;
    /**
     * Abort the stream.
     */
    abort(): void;
    [Symbol.asyncIterator](): AsyncIterator<T>;
    /**
     * Parse SSE format and extract data.
     * Returns null for [DONE] or empty events.
     *
     * SSE format:
     * - Lines starting with `:` are comments and are ignored
     * - `data:` lines contain the event payload
     * - Empty lines separate events
     * - We only process `data:` fields, ignoring `event:`, `id:`, `retry:`
     */
    private parseSSE;
    /**
     * Collect all events into an array.
     */
    toArray(): Promise<T[]>;
}

/** HTTP methods routable through `objectiveai api ...`. */
type ViewerHttpMethod = "GET" | "POST" | "DELETE";
/**
 * Start a cli-invoke session against the host. Returns an
 * AsyncIterable of `notification.value` payloads. The iterator
 * terminates on `{"type":"end"}` and throws an
 * [`ObjectiveAIFetchError`] on any `{"type":"error"}` line.
 */
declare function viewerApiCallChunks<T>(method: ViewerHttpMethod, path: string, body: unknown): AsyncIterable<T>;
/**
 * Make a viewer-mode unary API call. Awaits the first `notification`
 * envelope and returns its value; subsequent notifications are
 * silently discarded (unary endpoints emit exactly one). Throws on
 * `error` envelopes or if no notification arrives before `end`.
 */
declare function viewerApiCallUnary<T>(method: ViewerHttpMethod, path: string, body: unknown): Promise<T>;
/**
 * Make a viewer-mode unary API call with no expected response body.
 * Drains the iterator without inspecting any chunk values; throws on
 * `error` envelopes.
 */
declare function viewerApiCallUnaryNoResponse(method: ViewerHttpMethod, path: string, body: unknown): Promise<void>;
/**
 * Make a viewer-mode streaming API call. Returns a `Stream<T>`
 * backed by the chunk iterator so consumer code that does
 * `for await (const chunk of stream)` works identically to fetch
 * mode.
 */
declare function viewerApiCallStream<T>(method: ViewerHttpMethod, path: string, body: unknown): Stream<T>;

declare const ViewerEventSchema: z.ZodUnion<readonly [z.ZodObject<{
    destination: z.ZodString;
    sub_type: z.ZodString;
    type: z.ZodLiteral<"inbound">;
    value: z.ZodType<JsonValue, unknown, z.core.$ZodTypeInternals<JsonValue, unknown>>;
}, z.core.$strip>, z.ZodObject<{
    destination: z.ZodString;
    type: z.ZodLiteral<"cli_command">;
    value: z.ZodType<JsonValue, unknown, z.core.$ZodTypeInternals<JsonValue, unknown>>;
}, z.core.$strip>]>;
type ViewerEvent = z.infer<typeof ViewerEventSchema>;

/**
 * Register a handler for incoming `inbound` plugin events. Returns
 * an unsubscribe function.
 *
 * `sub_type` matches the `sub_type` field of the `Event::Inbound`
 * emitted by the Rust host — the string the plugin author
 * registered in their manifest's `viewer_routes` entry, or one of
 * the built-in event names (e.g. `agent_completions`).
 *
 * In iframe context the events come from the host's bridge; in
 * standalone-dev context they come from `@tauri-apps/api`'s `listen`.
 */
declare function listen<T = unknown>(sub_type: string, handler: (value: T) => void): () => void;
/**
 * Invoke `objectiveai-cli` in-process on the host with `args`.
 * Returns an `AsyncIterable<unknown>` over the JSONL output lines
 * the cli emits. Each yielded value is the parsed cli output
 * envelope (e.g. `{"type":"begin"}`, `{"type":"notification","value":...}`,
 * `{"type":"end"}`); the iterator terminates after the `end` line.
 *
 * The plugin author never specifies `destination` — the host bridge
 * derives it from the iframe's identity. To run multiple cli
 * invocations sequentially, `await` each iterator to completion
 * before starting the next; concurrent invocations from the same
 * iframe produce interleaved streams that this shim cannot demux
 * (no per-invocation sub_type on `cli_command` events).
 *
 * Requires the host viewer to be built with the `cli` Cargo feature.
 * Without it, the host's `cli_run` Tauri command isn't registered
 * and the invocation falls through to a Tauri "command not found"
 * error (silently ignored here — the iterator just yields nothing).
 */
declare function invokeCli(args: string[]): AsyncIterable<unknown>;
/** Internal-use: clear in-flight state. Exposed for tests only.
 * Note: the module-level `message` event listener stays attached —
 * removing/re-attaching it would just register a duplicate. The
 * listeners map is what carries per-test state. */
declare function __resetForTests(): void;

export { type JsonValue as J, Stream as S, type ViewerEvent as V, __resetForTests as _, ViewerEventSchema as a, type ViewerHttpMethod as b, viewerApiCallStream as c, viewerApiCallUnary as d, viewerApiCallUnaryNoResponse as e, invokeCli as i, listen as l, viewerApiCallChunks as v };
