/**
 * JS-side bridge for the `viewer: true` mode of the `ObjectiveAI`
 * client.
 *
 * When `client.viewer === true`, every HTTP method on the client
 * routes through this module instead of `fetch()`. Internally we
 * spawn `objectiveai api <segments> <method>` via the host viewer's
 * `cli_run` Tauri command (through the existing `invokeCli` shim)
 * and unwrap the cli's JSONL `Output<T>` envelopes:
 *
 *   - `{type:"begin"}` / `{type:"end"}` — markers, dropped here
 *   - `{type:"notification","value":<T>}` — yielded as a chunk
 *   - `{type:"error","level":..., "message":...}` — thrown as
 *     [`ObjectiveAIFetchError`]
 *
 * Unary endpoints emit one `notification` line; streaming endpoints
 * emit one `notification` per chunk. Either path collapses cleanly
 * onto the same iterator.
 *
 * The function exports here are the only viewer-mode entry points
 * the rest of the SDK ever calls — `client.ts`'s `get/post_unary` /
 * `get/post/delete_streaming` etc. all hit `viewerApiCall*`. Keeping
 * the names stable preserves that surface as we swap the underlying
 * transport from `api_call_run` (HTTP via SDK) to `cli_run`
 * (subprocess via `objectiveai` cli).
 */
import { ObjectiveAIFetchError } from "../error";
import { Stream } from "../stream";
import { invokeCli } from "./index";

/** HTTP methods routable through `objectiveai api ...`. */
export type ViewerHttpMethod = "GET" | "POST" | "DELETE";

/**
 * Map `(method, path)` to the cli argv vector. Convention matches
 * the coverage test in `objectiveai-cli/tests/api_endpoint_coverage.rs`:
 * URL segments become positional subcommands, the lowercased method
 * is the leaf verb (`post`/`get`/`delete`).
 */
function buildCliArgs(
  method: ViewerHttpMethod,
  path: string,
  body: unknown,
): string[] {
  const segments = path
    .replace(/^\//, "")
    .split("/")
    .filter((s) => s.length > 0);
  const args = ["objectiveai", "api", ...segments, method.toLowerCase()];
  if (body !== null && body !== undefined) {
    args.push("--body-inline", JSON.stringify(body));
  }
  return args;
}

/**
 * Shape of a single cli output envelope on the wire (the JSON value
 * inside one `cli_command` postMessage). Matches
 * `objectiveai_sdk::cli::output::Output<T>`'s serde-tagged form.
 */
type CliOutput =
  | { type: "begin" }
  | { type: "end" }
  | { type: "notification"; value: unknown }
  | { type: "error"; level?: string; fatal?: boolean; message?: unknown };

/**
 * Start a cli-invoke session against the host. Returns an
 * AsyncIterable of `notification.value` payloads. The iterator
 * terminates on `{"type":"end"}` and throws an
 * [`ObjectiveAIFetchError`] on any `{"type":"error"}` line.
 */
export function viewerApiCallChunks<T>(
  method: ViewerHttpMethod,
  path: string,
  body: unknown,
): AsyncIterable<T> {
  const args = buildCliArgs(method, path, body);
  const cliOutput = invokeCli(args);

  return {
    [Symbol.asyncIterator]() {
      const inner = cliOutput[Symbol.asyncIterator]();
      return {
        async next(): Promise<IteratorResult<T>> {
          for (;;) {
            const result = await inner.next();
            if (result.done) {
              return { value: undefined, done: true };
            }
            const line = result.value as CliOutput | null;
            if (!line || typeof line !== "object") continue;
            switch (line.type) {
              case "begin":
              case "end":
                continue;
              case "notification":
                return { value: line.value as T, done: false };
              case "error": {
                const message =
                  typeof line.message === "string"
                    ? line.message
                    : JSON.stringify(line.message ?? line);
                throw new ObjectiveAIFetchError(0, message);
              }
              default:
                continue;
            }
          }
        },
        async return(): Promise<IteratorResult<T>> {
          if (inner.return) {
            await inner.return();
          }
          return { value: undefined, done: true };
        },
      };
    },
  };
}

/**
 * Make a viewer-mode unary API call. Awaits the first `notification`
 * envelope and returns its value; subsequent notifications are
 * silently discarded (unary endpoints emit exactly one). Throws on
 * `error` envelopes or if no notification arrives before `end`.
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
