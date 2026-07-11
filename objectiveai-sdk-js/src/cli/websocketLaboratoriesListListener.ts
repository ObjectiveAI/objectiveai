/**
 * Materialized consumer of the cli daemon's `/laboratories/list`
 * endpoint over a NATIVE WebSocket — the JS mirror of the Rust SDK's
 * `cli::websocket_laboratories_list_listener::WebSocketLaboratoriesListListener`,
 * identical in construction and semantics.
 *
 * Each item is one laboratory's spec plus its `source` (local/remote
 * provenance, the unary `laboratories list` rules) and a live
 * `connected` flag from the daemon's `/laboratory` registry. The
 * listener folds events into a self-updating `id → status` map: a
 * `snapshot` replaces the whole set, `upserted` replaces one
 * laboratory by id (introducing it if unseen), `removed` drops one.
 * Per-lab attachment detail lives on the `/laboratories/{id}`
 * endpoint ({@link WebSocketLaboratoriesListener}).
 *
 * Auth is the first-message preamble shared by every daemon WebSocket
 * route. One listener = one connection: when the socket closes the
 * view freezes at its last state; reconnection is the caller's loop.
 * Unparseable events are skipped (forward compat). No runtime
 * validation, like {@link WebSocketListener}.
 */

import type {
  CliWebsocketLaboratoriesListListenerLaboratoryEvent,
  CliWebsocketLaboratoriesListListenerLaboratoryStatus,
} from "./websocket_laboratories_list_listener";

type LaboratoryStatus = CliWebsocketLaboratoriesListListenerLaboratoryStatus;
type LaboratoryEvent = CliWebsocketLaboratoriesListListenerLaboratoryEvent;

export interface WebSocketLaboratoriesListListenerOptions {
  /** The pre-derived `sha256=<hex(SHA256(DAEMON_SECRET))>`, sent
   * verbatim in the auth preamble. Without it the daemon must be
   * running without a secret. */
  signature?: string | null;
  /** Invoked with the full current laboratory set (sorted by id)
   * after every applied change. Runs synchronously on frame receipt,
   * so keep it cheap; for the full state on demand use
   * {@link WebSocketLaboratoriesListListener.laboratories}. */
  onChange?: (laboratories: LaboratoryStatus[]) => void;
}

/**
 * The materialized `/laboratories/list` view — construct via
 * {@link WebSocketLaboratoriesListListener.connect}.
 *
 * ```ts
 * const listener = await WebSocketLaboratoriesListListener.connect(
 *   "ws://127.0.0.1:49152/laboratories/list",
 *   { signature, onChange: (laboratories) => render(laboratories) },
 * );
 * listener.laboratories(); // [{ id, image, ..., source, connected }, ...]
 * await listener.subscribe(); // resolves on the next change
 * ```
 */
export class WebSocketLaboratoriesListListener {
  #ws: WebSocket;
  #closed = false;
  /** `id → status`. */
  #state = new Map<string, LaboratoryStatus>();
  #onChange?: (laboratories: LaboratoryStatus[]) => void;
  /** Resolvers for the in-flight {@link subscribe} promises. */
  #waiters = new Set<() => void>();

  private constructor(
    ws: WebSocket,
    onChange?: (laboratories: LaboratoryStatus[]) => void,
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
   * the daemon's full `/laboratories/list` URL. The returned listener
   * immediately begins folding events (the first is the connect-time
   * snapshot).
   */
  static connect(
    url: string,
    options?: WebSocketLaboratoriesListListenerOptions,
  ): Promise<WebSocketLaboratoriesListListener> {
    return new Promise((resolve, reject) => {
      let ws: WebSocket;
      try {
        ws = new WebSocket(url);
      } catch (e) {
        reject(
          new Error(`connect daemon laboratories websocket: ${String(e)}`),
        );
        return;
      }
      ws.onopen = () => {
        // The auth preamble — always the connection's first text frame,
        // `{"signature": null}` against a secretless daemon.
        ws.send(JSON.stringify({ signature: options?.signature ?? null }));
        resolve(
          new WebSocketLaboratoriesListListener(ws, options?.onChange),
        );
      };
      ws.onclose = (event: CloseEvent) => {
        reject(
          new Error(
            `connect daemon laboratories websocket: connection closed` +
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

  /** Snapshot the current laboratory set, sorted by `id`. */
  laboratories(): LaboratoryStatus[] {
    return [...this.#state.entries()]
      .sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))
      .map(([, laboratory]) => laboratory);
  }

  /** Resolves on the next change applied to the state. A fresh call
   * waits for the FIRST change after it is made — loop with the
   * {@link laboratories} read, or use
   * {@link WebSocketLaboratoriesListListenerOptions.onChange} for
   * guaranteed push. Resolves immediately if already closed. */
  subscribe(): Promise<void> {
    if (this.#closed) return Promise.resolve();
    return new Promise<void>((resolve) => {
      this.#waiters.add(resolve);
    });
  }

  #onFrame(frame: unknown): void {
    if (typeof frame !== "object" || frame === null) return;
    if (typeof (frame as { type?: unknown }).type !== "string") return;
    if (this.#apply(frame as LaboratoryEvent)) {
      this.#notify();
    }
  }

  /** Fold one event into the map. `true` when a known event was
   * applied (an unrecognized `type` is a no-op — no spurious change). */
  #apply(event: LaboratoryEvent): boolean {
    switch (event.type) {
      case "snapshot":
        this.#state.clear();
        for (const laboratory of event.laboratories) {
          this.#state.set(laboratory.id, laboratory);
        }
        return true;
      case "upserted":
        this.#state.set(event.laboratory.id, event.laboratory);
        return true;
      case "removed":
        this.#state.delete(event.id);
        return true;
      default:
        return false;
    }
  }

  /** Fire the callback with the refreshed set and wake every
   * {@link subscribe} waiter. */
  #notify(): void {
    if (this.#onChange) {
      this.#onChange(this.laboratories());
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
