import type { CliCommandRequest } from "../request";

/**
 * The FIRST client→daemon message on EVERY daemon WebSocket
 * connection — always sent, even to a secretless daemon
 * (`{"signature": null}`). The JS mirror of the Rust SDK's
 * `cli.command.command_executor.AuthEnvelope`; headers are never used
 * for auth because browser WebSocket clients can't set them.
 */
interface AuthEnvelope {
  signature: string | null;
}

/**
 * The one request message on an `/execute` connection, sent right
 * after the auth preamble — the JS mirror of the Rust SDK's
 * `cli.command.command_executor.ExecuteEnvelope`.
 */
interface ExecuteEnvelope {
  agent_arguments?: Record<string, string | null>;
  request: CliCommandRequest;
}

export interface WebSocketExecutorOptions {
  /** The pre-derived `sha256=<hex(SHA256(DAEMON_SECRET))>`, sent
   * verbatim in the auth preamble. Without it the daemon must be
   * running without a secret. */
  signature?: string | null;
  /** Per-request identity override, sent in every execute envelope
   * (same semantics as the other executors' override;
   * `mcp_session_id` is ignored server-side). */
  agentArguments?: Record<string, string | null>;
}

/**
 * Execute commands against a cli daemon over a NATIVE WebSocket —
 * connection per command on the daemon's `/execute` route. The JS
 * mirror of the Rust SDK's `WebSocketExecutor`, usable anywhere a
 * `WebSocket` can reach the daemon (the main viewer window, Node 22+,
 * any browser context with network access).
 *
 * Wire contract (one connection per `execute`):
 * - client sends TWO text messages — the auth preamble
 *   (`{"signature": …}`), then the execute envelope
 *   (`{agent_arguments?, request}`) — then only reads;
 * - the daemon sends one text message per stream item, exactly the
 *   cli's stdout JSONL line shapes (a response JSON or a
 *   `{"type":"error",…}` line), then closes: the close IS the
 *   end-of-stream marker, so iteration simply ends (no synthetic
 *   `{"type":"end"}` — binary-executor parity);
 * - a transport failure (connect error, abnormal close) surfaces as
 *   one in-band `{"type":"error",…}` line, then the iterator ends;
 * - `return()`/`break` closes the socket, which cancels the
 *   daemon-side run.
 */
export class WebSocketExecutor {
  readonly #url: string;
  readonly #signature: string | null;
  readonly #agentArguments: Record<string, string | null> | undefined;

  /** `url` is the full connect URL of the daemon's execute route,
   * e.g. `ws://127.0.0.1:49152/execute`. */
  constructor(url: string, options?: WebSocketExecutorOptions) {
    this.#url = url;
    this.#signature = options?.signature ?? null;
    this.#agentArguments = options?.agentArguments;
  }

  execute(request: CliCommandRequest): AsyncIterable<unknown> {
    const url = this.#url;
    const auth: AuthEnvelope = { signature: this.#signature };
    const envelope: ExecuteEnvelope = {
      ...(this.#agentArguments !== undefined
        ? { agent_arguments: this.#agentArguments }
        : {}),
      request,
    };
    return {
      [Symbol.asyncIterator]() {
        const queue: unknown[] = [];
        let resolveNext: (() => void) | null = null;
        let done = false;
        let socket: WebSocket | null = null;

        const wake = () => {
          if (resolveNext) {
            const r = resolveNext;
            resolveNext = null;
            r();
          }
        };
        const push = (value: unknown) => {
          if (done) return;
          queue.push(value);
          wake();
        };
        const end = () => {
          done = true;
          wake();
        };
        /** Transport failure: one in-band error line, then end. */
        const fail = (message: string) => {
          push({ type: "error", level: "error", fatal: null, message });
          end();
        };

        try {
          socket = new WebSocket(url);
        } catch (e) {
          fail(`connect daemon execute websocket: ${String(e)}`);
        }
        if (socket) {
          const ws = socket;
          let opened = false;
          ws.onopen = () => {
            opened = true;
            // The auth preamble, then the one request envelope.
            ws.send(JSON.stringify(auth));
            ws.send(JSON.stringify(envelope));
          };
          ws.onmessage = (event: MessageEvent) => {
            if (typeof event.data !== "string") return;
            try {
              push(JSON.parse(event.data));
            } catch (e) {
              push({
                type: "error",
                level: "error",
                fatal: null,
                message: `decode daemon execute message: ${String(e)}`,
              });
            }
          };
          ws.onerror = () => {
            // The close handler follows and reports; nothing to do
            // here (browser error events carry no detail anyway).
          };
          ws.onclose = (event: CloseEvent) => {
            if (done) return;
            // A close before the handshake completed means the daemon
            // was unreachable or refused us (bad auth closes without
            // a word) — surface it. A close after open is the normal
            // end-of-stream marker.
            if (!opened) {
              fail(
                `connect daemon execute websocket: connection closed` +
                  `${event.code ? ` (code ${event.code})` : ""}`,
              );
              return;
            }
            end();
          };
        }

        const finish = () => {
          if (
            socket &&
            socket.readyState !== WebSocket.CLOSED &&
            socket.readyState !== WebSocket.CLOSING
          ) {
            // Dropping the connection cancels the daemon-side run.
            socket.close();
          }
          done = true;
        };

        return {
          async next(): Promise<IteratorResult<unknown>> {
            while (queue.length === 0 && !done) {
              await new Promise<void>((r) => {
                resolveNext = r;
              });
            }
            if (queue.length > 0) {
              return { value: queue.shift(), done: false };
            }
            finish();
            return { value: undefined, done: true };
          },
          async return(): Promise<IteratorResult<unknown>> {
            finish();
            return { value: undefined, done: true };
          },
        };
      },
    };
  }
}
