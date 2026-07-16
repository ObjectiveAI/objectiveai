import type { CliCommandRequest } from "../request";
import { connectViewerStream, type ViewerTransport } from "../../viewer";

/**
 * Execute commands against the cli daemon through the viewer's Tauri
 * IPC proxy — the viewer-mode sibling of {@link SseCommandExecutor}.
 * One `daemon_execute` invoke per command; the Rust side POSTs the
 * request to the daemon's `/execute` and streams the raw result lines
 * back over the channel. No url, no signature, no identity: the Rust
 * proxy owns the address and stamps the auth signature + the viewer
 * agent identity server-side.
 *
 * Stream contract (identical to `SseCommandExecutor.execute`):
 * - one item per daemon stream line — a response JSON or a
 *   `{"type":"error",…}` line; the stream ending IS the end-of-stream
 *   marker (no synthetic `{"type":"end"}`);
 * - a connect failure surfaces as one in-band `{"type":"error",…}`
 *   line, then the iterator ends;
 * - `return()`/`break` cancels the Rust-side connection, which
 *   cancels the daemon-side run.
 */
export class ViewerCommandExecutor {
  readonly #transport: ViewerTransport;

  constructor(transport: ViewerTransport) {
    this.#transport = transport;
  }

  execute(request: CliCommandRequest): AsyncIterable<unknown> {
    const transport = this.#transport;
    return {
      async *[Symbol.asyncIterator]() {
        const controller = new AbortController();
        let events: AsyncGenerator<string>;
        try {
          events = await connectViewerStream(
            transport,
            "daemon_execute",
            { request: JSON.stringify(request) },
            controller.signal,
          );
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
            message: `daemon execute stream: ${String(e)}`,
          };
        } finally {
          // return()/break (or normal completion) cancels the
          // Rust-side connection, cancelling the daemon-side run.
          controller.abort();
        }
      },
    };
  }
}
