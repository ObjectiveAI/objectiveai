import type { CliCommandRequest } from "../request";
import { connectSse } from "../../sse";

export interface SseCommandExecutorOptions {
  /** The pre-derived `sha256=<hex(SHA256(DAEMON_SECRET))>`, sent as the
   * `X-OBJECTIVEAI-SIGNATURE` header. Without it the daemon must be
   * running without a secret. */
  signature?: string | null;
  /** Value for the `X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY` header. */
  agentInstanceHierarchy?: string | null;
  /** Value for the `X-OBJECTIVEAI-AGENT-ID` header. */
  agentId?: string | null;
  /** Value for the `X-OBJECTIVEAI-AGENT-FULL-ID` header. */
  agentFullId?: string | null;
  /** Value for the `X-OBJECTIVEAI-AGENT-REMOTE` header (the
   * JSON-encoded `RemotePath`). */
  agentRemote?: string | null;
  /** Value for the `X-OBJECTIVEAI-RESPONSE-ID` header. */
  responseId?: string | null;
  /** Value for the `X-OBJECTIVEAI-RESPONSE-IDS` header. */
  responseIds?: string | null;
}

/**
 * Execute commands against a cli daemon over plain HTTP — one POST per
 * command to the daemon's `/execute` route, the result streamed back as
 * Server-Sent Events. The JS mirror of the Rust SDK's
 * `SseCommandExecutor`, usable anywhere `fetch` can reach the daemon
 * (the main viewer window, Node 22+, any browser context with network
 * access).
 *
 * Wire contract (one request per `execute`):
 * - client POSTs the request's JSON as the raw body — nothing wraps
 *   it — with the auth signature in the `X-OBJECTIVEAI-SIGNATURE`
 *   header and the identity override in the `X-OBJECTIVEAI-*` agent
 *   headers, one header per set option ({@link
 *   SseCommandExecutorOptions}). A missing header DELETES that
 *   config field on the daemon for the run — never inherits;
 * - the daemon replies `text/event-stream`, one SSE `data:` event per
 *   stream item — exactly the cli's stdout JSONL line shapes (a
 *   response JSON or a `{"type":"error",…}` line); the response body
 *   ending IS the end-of-stream marker, so iteration simply ends (no
 *   synthetic `{"type":"end"}` — binary-executor parity);
 * - a connection failure / non-2xx status (e.g. `401` on a bad
 *   signature) surfaces as one in-band `{"type":"error",…}` line, then
 *   the iterator ends;
 * - `return()`/`break` aborts the request, which cancels the
 *   daemon-side run.
 */
export class SseCommandExecutor {
  readonly #url: string;
  readonly #options: SseCommandExecutorOptions | undefined;

  /** `url` is the full URL of the daemon's execute route, e.g.
   * `http://127.0.0.1:49152/execute`. */
  constructor(url: string, options?: SseCommandExecutorOptions) {
    this.#url = url;
    this.#options = options;
  }

  execute(request: CliCommandRequest): AsyncIterable<unknown> {
    const url = this.#url;
    const options = this.#options;
    return {
      async *[Symbol.asyncIterator]() {
        const controller = new AbortController();
        let events: AsyncGenerator<string>;
        try {
          events = await connectSse(url, options?.signature, controller.signal, {
            method: "POST",
            body: JSON.stringify(request),
            agentInstanceHierarchy: options?.agentInstanceHierarchy,
            agentId: options?.agentId,
            agentFullId: options?.agentFullId,
            agentRemote: options?.agentRemote,
            responseId: options?.responseId,
            responseIds: options?.responseIds,
          });
        } catch (e) {
          yield {
            type: "error",
            level: "error",
            fatal: null,
            message: `connect daemon execute: ${String(e)}`,
          };
          return;
        }
        try {
          for await (const data of events) {
            try {
              yield JSON.parse(data);
            } catch (e) {
              yield {
                type: "error",
                level: "error",
                fatal: null,
                message: `decode daemon execute message: ${String(e)}`,
              };
            }
          }
        } catch (e) {
          yield {
            type: "error",
            level: "error",
            fatal: null,
            message: `daemon execute sse stream: ${String(e)}`,
          };
        } finally {
          // return()/break (or normal completion) aborts the request,
          // cancelling the daemon-side run.
          controller.abort();
        }
      },
    };
  }
}
