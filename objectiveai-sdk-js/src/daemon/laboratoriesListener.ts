/**
 * Materialized consumer of the cli daemon's `/laboratories/{id}`
 * endpoint — the JS mirror of the Rust SDK's
 * `daemon::laboratories_listener::LaboratoriesListener`,
 * identical in semantics.
 *
 * One laboratory's full record: its spec (when a connected host
 * serves it; zero-filled otherwise), `machine`/`connected`
 * state, and EVERY attachment row targeting it (the AIHs and tags it
 * is attached to). Frames are always full-value record replaces —
 * the first is the connect-time snapshot; each later one supersedes
 * it wholesale.
 *
 * One listener = one connection: when the socket closes the view
 * freezes at its last state; reconnect is the caller's loop — mint a
 * new listener from the client. Unparseable events are skipped
 * (forward compat). No runtime validation, like
 * {@link BroadcastListener}.
 */

import type {
  DaemonLaboratoriesListenerLaboratoryInstanceEvent,
  DaemonLaboratoriesListenerLaboratoryRecord,
} from "./laboratories_listener";
import { openStream, type ClientMode } from "./mode";

type LaboratoryRecord = DaemonLaboratoriesListenerLaboratoryRecord;
type LaboratoryInstanceEvent =
  DaemonLaboratoriesListenerLaboratoryInstanceEvent;

/**
 * The materialized `/laboratories/{id}` view — minted by
 * {@link Client.laboratoriesListener}; resolves only once the stream
 * has OPENED.
 *
 * ```ts
 * const listener = await client.laboratoriesListener(id);
 * listener.laboratory(); // { id, ..., connected, attachments } | null
 * await listener.subscribe(); // resolves on the next change
 * ```
 */
export class LaboratoriesListener {
  #abort: AbortController;
  #closed = false;
  /** The latest full record — `null` until the first frame lands. */
  #state: LaboratoryRecord | null = null;
  /** Resolvers for the in-flight {@link subscribe} promises. */
  #waiters = new Set<() => void>();

  private constructor(abort: AbortController) {
    this.#abort = abort;
  }

  /** @internal — minted by `Client.laboratoriesListener`.
   * `laboratoryId` is the raw laboratory id; the optional
   * `(machine, machineState)` pair pins the exact owning host (both
   * or neither). */
  static async _connect(
    mode: ClientMode,
    laboratoryId: string,
    options?: { machine?: string; machineState?: string },
  ): Promise<LaboratoriesListener> {
    const abort = new AbortController();
    const events = await openStream(mode, abort.signal, {
      route: `/laboratories/${laboratoryId}`,
      query: {
        machine: options?.machine,
        machine_state: options?.machineState,
      },
      viewerCommand: "daemon_laboratory",
      viewerArgs: {
        id: laboratoryId,
        machine: options?.machine,
        machineState: options?.machineState,
      },
    });
    const listener = new LaboratoriesListener(abort);
    void listener.#pump(events);
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
   * {@link laboratory} read. Resolves immediately if already closed. */
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
