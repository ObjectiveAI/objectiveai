/**
 * Materialized consumer of the cli daemon's `/laboratories/{id}`
 * endpoint over Server-Sent Events (SSE) — the JS mirror of the Rust SDK's
 * `cli::websocket_laboratories_listener::WebSocketLaboratoriesListener`,
 * identical in construction and semantics.
 *
 * One laboratory's full record: its spec (when a connected host
 * serves it; zero-filled otherwise), `machine`/`connected`
 * state, and EVERY attachment row targeting it (the AIHs and tags it
 * is attached to). Frames are always full-value record replaces —
 * the first is the connect-time snapshot; each later one supersedes
 * it wholesale.
 *
 * Auth rides the `X-OBJECTIVEAI-SIGNATURE` request header. One listener = one connection: when the socket closes the
 * view freezes at its last state; reconnection is the caller's loop.
 * Unparseable events are skipped (forward compat). No runtime
 * validation, like {@link WebSocketListener}.
 */

import { connectSse } from "./sse";

import type {
  CliWebsocketLaboratoriesListenerLaboratoryInstanceEvent,
  CliWebsocketLaboratoriesListenerLaboratoryRecord,
} from "./websocket_laboratories_listener";

type LaboratoryRecord = CliWebsocketLaboratoriesListenerLaboratoryRecord;
type LaboratoryInstanceEvent =
  CliWebsocketLaboratoriesListenerLaboratoryInstanceEvent;

export interface WebSocketLaboratoriesListenerOptions {
  /** The pre-derived `sha256=<hex(SHA256(DAEMON_SECRET))>`, sent
   * as the `X-OBJECTIVEAI-SIGNATURE` header. Without it the daemon must be
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
  #abort: AbortController;
  #closed = false;
  /** The latest full record — `null` until the first frame lands. */
  #state: LaboratoryRecord | null = null;
  #onChange?: (laboratory: LaboratoryRecord) => void;
  /** Resolvers for the in-flight {@link subscribe} promises. */
  #waiters = new Set<() => void>();

  private constructor(
    abort: AbortController,
    onChange?: (laboratory: LaboratoryRecord) => void,
  ) {
    this.#abort = abort;
    this.#onChange = onChange;
  }

  /**
   * Open the connection, send the auth preamble, and resolve once the
   * socket is established (rejects when the daemon is unreachable or
   * closes during the handshake — e.g. it refused the auth). `url` is
   * the daemon's full per-laboratory URL (`/laboratories/` + the raw
   * laboratory id). The returned listener immediately begins folding
   * frames (the first is the connect-time record).
   */
  static async connect(
    url: string,
    options?: WebSocketLaboratoriesListenerOptions,
  ): Promise<WebSocketLaboratoriesListener> {
    const abort = new AbortController();
    let events: AsyncGenerator<string>;
    try {
      events = await connectSse(url, options?.signature, abort.signal);
    } catch (e) {
      throw new Error(`connect daemon laboratory sse: ${String(e)}`);
    }
    const listener = new WebSocketLaboratoriesListener(abort, options?.onChange);
    listener.#pump(events);
    return listener;
  }

  /** Read the SSE stream, feeding each event to {@link #onFrame} until
   * it ends (or is aborted); then settle as closed. */
  async #pump(events: AsyncGenerator<string>): Promise<void> {
    try {
      for await (const data of events) {
        let frame: unknown;
        try {
          frame = JSON.parse(data);
        } catch {
          continue;
        }
        this.#onFrame(frame);
      }
    } catch {
      // Aborted or transport error — fall through to close.
    } finally {
      this.#onClose();
    }
  }

  /** Whether the connection has closed (the view is frozen). */
  get closed(): boolean {
    return this.#closed;
  }

  /** Drop the connection: the view freezes and any pending
   * {@link subscribe} resolves. */
  close(): void {
    if (this.#closed) return;
    this.#abort.abort();
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
