/**
 * Materialized consumer of the cli daemon's `/laboratories/{id}`
 * endpoint over a NATIVE WebSocket — the JS mirror of the Rust SDK's
 * `cli::websocket_laboratories_listener::WebSocketLaboratoriesListener`,
 * identical in construction and semantics.
 *
 * One laboratory's full record: its spec (when connected or in the
 * machine's local scan; zero-filled otherwise), `source`/`connected`
 * state, and EVERY attachment row targeting it (the AIHs and tags it
 * is attached to). Frames are always full-value record replaces —
 * the first is the connect-time snapshot; each later one supersedes
 * it wholesale.
 *
 * Auth is the first-message preamble shared by every daemon WebSocket
 * route. One listener = one connection: when the socket closes the
 * view freezes at its last state; reconnection is the caller's loop.
 * Unparseable events are skipped (forward compat). No runtime
 * validation, like {@link WebSocketListener}.
 */

import type {
  CliWebsocketLaboratoriesListenerLaboratoryInstanceEvent,
  CliWebsocketLaboratoriesListenerLaboratoryRecord,
} from "./websocket_laboratories_listener";

type LaboratoryRecord = CliWebsocketLaboratoriesListenerLaboratoryRecord;
type LaboratoryInstanceEvent =
  CliWebsocketLaboratoriesListenerLaboratoryInstanceEvent;

export interface WebSocketLaboratoriesListenerOptions {
  /** The pre-derived `sha256=<hex(SHA256(DAEMON_SECRET))>`, sent
   * verbatim in the auth preamble. Without it the daemon must be
   * running without a secret. */
  signature?: string | null;
  /** Invoked with the fresh full record after every applied frame.
   * Runs synchronously on frame receipt, so keep it cheap; for the
   * state on demand use
   * {@link WebSocketLaboratoriesListener.laboratory}. */
  onChange?: (laboratory: LaboratoryRecord) => void;
}

/**
 * The materialized `/laboratories/{id}` view — construct via
 * {@link WebSocketLaboratoriesListener.connect}.
 *
 * ```ts
 * const listener = await WebSocketLaboratoriesListener.connect(
 *   `ws://127.0.0.1:49152/laboratories/${id}`,
 *   { signature, onChange: (laboratory) => render(laboratory) },
 * );
 * listener.laboratory(); // { id, ..., connected, attachments } | null
 * await listener.subscribe(); // resolves on the next change
 * ```
 */
export class WebSocketLaboratoriesListener {
  #ws: WebSocket;
  #closed = false;
  /** The latest full record — `null` until the first frame lands. */
  #state: LaboratoryRecord | null = null;
  #onChange?: (laboratory: LaboratoryRecord) => void;
  /** Resolvers for the in-flight {@link subscribe} promises. */
  #waiters = new Set<() => void>();

  private constructor(
    ws: WebSocket,
    onChange?: (laboratory: LaboratoryRecord) => void,
  ) {
    this.#ws = ws;
    this.#onChange = onChange;
    ws.onmessage = (event: MessageEvent) => {
      if (typeof event.data !== "string") return;
      let frame: unknown;
      try {
        frame = JSON.parse(event.data);
      } catch {
        return;
      }
      this.#onFrame(frame);
    };
    ws.onclose = () => this.#onClose();
  }

  /**
   * Open the connection, send the auth preamble, and resolve once the
   * socket is established (rejects when the daemon is unreachable or
   * closes during the handshake — e.g. it refused the auth). `url` is
   * the daemon's full per-laboratory URL (`/laboratories/` + the raw
   * laboratory id). The returned listener immediately begins folding
   * frames (the first is the connect-time record).
   */
  static connect(
    url: string,
    options?: WebSocketLaboratoriesListenerOptions,
  ): Promise<WebSocketLaboratoriesListener> {
    return new Promise((resolve, reject) => {
      let ws: WebSocket;
      try {
        ws = new WebSocket(url);
      } catch (e) {
        reject(
          new Error(`connect daemon laboratory websocket: ${String(e)}`),
        );
        return;
      }
      ws.onopen = () => {
        // The auth preamble — always the connection's first text frame,
        // `{"signature": null}` against a secretless daemon.
        ws.send(JSON.stringify({ signature: options?.signature ?? null }));
        resolve(new WebSocketLaboratoriesListener(ws, options?.onChange));
      };
      ws.onclose = (event: CloseEvent) => {
        reject(
          new Error(
            `connect daemon laboratory websocket: connection closed` +
              `${event.code ? ` (code ${event.code})` : ""}`,
          ),
        );
      };
    });
  }

  /** Whether the connection has closed (the view is frozen). */
  get closed(): boolean {
    return this.#closed;
  }

  /** Drop the connection: the view freezes and any pending
   * {@link subscribe} resolves. */
  close(): void {
    if (this.#closed) return;
    this.#ws.close();
    // The browser fires onclose asynchronously; settle deterministically.
    this.#onClose();
  }

  /** The current record — `null` before the first frame. */
  laboratory(): LaboratoryRecord | null {
    return this.#state;
  }

  /** Resolves on the next frame applied to the state. A fresh call
   * waits for the FIRST change after it is made — loop with the
   * {@link laboratory} read, or use
   * {@link WebSocketLaboratoriesListenerOptions.onChange} for
   * guaranteed push. Resolves immediately if already closed. */
  subscribe(): Promise<void> {
    if (this.#closed) return Promise.resolve();
    return new Promise<void>((resolve) => {
      this.#waiters.add(resolve);
    });
  }

  #onFrame(frame: unknown): void {
    if (typeof frame !== "object" || frame === null) return;
    if ((frame as { type?: unknown }).type !== "laboratory") return;
    const event = frame as Extract<
      LaboratoryInstanceEvent,
      { type: "laboratory" }
    >;
    this.#state = event.laboratory;
    if (this.#onChange) {
      this.#onChange(event.laboratory);
    }
    const waiters = [...this.#waiters];
    this.#waiters.clear();
    for (const wake of waiters) wake();
  }

  #onClose(): void {
    if (this.#closed) return;
    this.#closed = true;
    const waiters = [...this.#waiters];
    this.#waiters.clear();
    for (const wake of waiters) wake();
  }
}
