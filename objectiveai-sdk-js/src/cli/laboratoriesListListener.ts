/**
 * Materialized consumer of the cli daemon's `/laboratories/list`
 * endpoint over Server-Sent Events (SSE) — the JS mirror of the Rust SDK's
 * `cli::laboratories_list_listener::LaboratoriesListListener`,
 * identical in construction and semantics.
 *
 * Each item is one laboratory's spec plus the `machine` whose host
 * serves it (there is no local-vs-remote split — machine identity is
 * the only provenance) and a live `connected` flag from the daemon's
 * `/laboratory` registry. The
 * listener folds events into a self-updating `id → status` map: a
 * `snapshot` replaces the whole set, `upserted` replaces one
 * laboratory by id (introducing it if unseen), `removed` drops one.
 * Per-lab attachment detail lives on the `/laboratories/{id}`
 * endpoint ({@link LaboratoriesListener}).
 *
 * Auth rides the `X-OBJECTIVEAI-SIGNATURE` request header. One listener = one connection: when the socket closes the
 * view freezes at its last state; reconnection is the caller's loop.
 * Unparseable events are skipped (forward compat). No runtime
 * validation, like {@link BroadcastListener}.
 */

import { connectSse } from "./sse";
import { connectViewerStream, type ViewerTransport } from "./viewer";

import type {
  CliLaboratoriesListListenerLaboratoryEvent,
  CliLaboratoriesListListenerLaboratoryStatus,
} from "./laboratories_list_listener";

type LaboratoryStatus = CliLaboratoriesListListenerLaboratoryStatus;
type LaboratoryEvent = CliLaboratoriesListListenerLaboratoryEvent;

export interface LaboratoriesListListenerOptions {
  /** The pre-derived `sha256=<hex(SHA256(DAEMON_SECRET))>`, sent
   * as the `X-OBJECTIVEAI-SIGNATURE` header. Without it the daemon must be
   * running without a secret. */
  signature?: string | null;
  /** Invoked with the full current laboratory set (sorted by id)
   * after every applied change. Runs synchronously on frame receipt,
   * so keep it cheap; for the full state on demand use
   * {@link LaboratoriesListListener.laboratories}. */
  onChange?: (laboratories: LaboratoryStatus[]) => void;
}

/** {@link LaboratoriesListListener.connectViewer} options — the
 * regular options sans `signature` (the Rust proxy stamps auth). */
export type LaboratoriesListListenerViewerOptions = Omit<
  LaboratoriesListListenerOptions,
  "signature"
>;

/**
 * The materialized `/laboratories/list` view — construct via
 * {@link LaboratoriesListListener.connect}.
 *
 * ```ts
 * const listener = await LaboratoriesListListener.connect(
 *   "http://127.0.0.1:49152/laboratories/list",
 *   { signature, onChange: (laboratories) => render(laboratories) },
 * );
 * listener.laboratories(); // [{ id, image, ..., machine, connected }, ...]
 * await listener.subscribe(); // resolves on the next change
 * ```
 */
/** The fold key: the full `(machine, state, id)` laboratory identity
 * (ids are only unique per (machine, state)). Empty segments when the
 * daemon didn't report the pair. */
function foldKey(
  id: string,
  machine?: string | null,
  machineState?: string | null,
): string {
  return `${machine ?? ""}\n${machineState ?? ""}\n${id}`;
}

export class LaboratoriesListListener {
  #abort: AbortController;
  #closed = false;
  /** `(machine, state, id) fold key → status` — laboratory ids are
   * only unique per (machine, state), so same-id items from different
   * hosts coexist. */
  #state = new Map<string, LaboratoryStatus>();
  #onChange?: (laboratories: LaboratoryStatus[]) => void;
  /** Resolvers for the in-flight {@link subscribe} promises. */
  #waiters = new Set<() => void>();

  private constructor(
    abort: AbortController,
    onChange?: (laboratories: LaboratoryStatus[]) => void,
  ) {
    this.#abort = abort;
    this.#onChange = onChange;
  }

  /**
   * Open the connection, send the auth preamble, and resolve once the
   * socket is established (rejects when the daemon is unreachable or
   * closes during the handshake — e.g. it refused the auth). `url` is
   * the daemon's full `/laboratories/list` URL. The returned listener
   * immediately begins folding events (the first is the connect-time
   * snapshot).
   */
  static async connect(
    url: string,
    options?: LaboratoriesListListenerOptions,
  ): Promise<LaboratoriesListListener> {
    const abort = new AbortController();
    let events: AsyncGenerator<string>;
    try {
      events = await connectSse(url, options?.signature, abort.signal);
    } catch (e) {
      throw new Error(`connect daemon laboratories sse: ${String(e)}`);
    }
    const listener = new LaboratoriesListListener(
      abort,
      options?.onChange,
    );
    listener.#pump(events);
    return listener;
  }

  /**
   * Viewer-mode connect: the stream rides the Tauri IPC proxy
   * ({@link connectViewerStream}) instead of fetch — no address, no
   * signature, no identity (the Rust side owns all three). Same
   * resolve/reject and lifecycle semantics as {@link connect};
   * reconnection remains the caller's loop.
   */
  static async connectViewer(
    transport: ViewerTransport,
    options?: LaboratoriesListListenerViewerOptions,
  ): Promise<LaboratoriesListListener> {
    const abort = new AbortController();
    let events: AsyncGenerator<string>;
    try {
      events = await connectViewerStream(
        transport,
        "daemon_laboratories_list",
        {},
        abort.signal,
      );
    } catch (e) {
      throw new Error(`connect daemon laboratories sse: ${String(e)}`);
    }
    const listener = new LaboratoriesListListener(
      abort,
      options?.onChange,
    );
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

  /** Snapshot the current laboratory set, sorted by `id`. */
  laboratories(): LaboratoryStatus[] {
    return [...this.#state.values()].sort((a, b) =>
      a.id < b.id ? -1 : a.id > b.id ? 1 : 0,
    );
  }

  /** Resolves on the next change applied to the state. A fresh call
   * waits for the FIRST change after it is made — loop with the
   * {@link laboratories} read, or use
   * {@link LaboratoriesListListenerOptions.onChange} for
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
          this.#state.set(
            foldKey(
              laboratory.id,
              laboratory.machine?.id,
              laboratory.machine_state,
            ),
            laboratory,
          );
        }
        return true;
      case "upserted":
        this.#state.set(
          foldKey(
            event.laboratory.id,
            event.laboratory.machine?.id,
            event.laboratory.machine_state,
          ),
          event.laboratory,
        );
        return true;
      case "removed":
        this.#state.delete(foldKey(event.id, event.machine, event.machine_state));
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
