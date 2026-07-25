/**
 * Materialized consumer of the cli daemon's `/laboratories/list`
 * endpoint — the JS mirror of the Rust SDK's
 * `daemon::laboratories_list_listener::LaboratoriesListListener`,
 * identical in semantics.
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
 * One listener = one connection: when the socket closes the view
 * freezes at its last state; reconnect is the caller's loop — mint a
 * new listener from the client. Unparseable events are skipped
 * (forward compat). No runtime validation, like
 * {@link BroadcastListener}.
 */

import type {
  DaemonLaboratoriesListListenerLaboratoryEvent,
  DaemonLaboratoriesListListenerLaboratoryStatus,
} from "./laboratories_list_listener";
import { openStream, type ClientMode } from "./mode";

type LaboratoryStatus = DaemonLaboratoriesListListenerLaboratoryStatus;
type LaboratoryEvent = DaemonLaboratoriesListListenerLaboratoryEvent;

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

/**
 * The materialized `/laboratories/list` view — minted by
 * {@link Client.laboratoriesListListener}; resolves only once the
 * stream has OPENED.
 *
 * ```ts
 * const listener = await client.laboratoriesListListener();
 * listener.laboratories(); // [{ id, image, ..., machine, connected }, ...]
 * await listener.subscribe(); // resolves on the next change
 * ```
 */
export class LaboratoriesListListener {
  #abort: AbortController;
  #closed = false;
  /** `(machine, state, id) fold key → status` — laboratory ids are
   * only unique per (machine, state), so same-id items from different
   * hosts coexist. */
  #state = new Map<string, LaboratoryStatus>();
  /** Resolvers for the in-flight {@link subscribe} promises. */
  #waiters = new Set<() => void>();

  private constructor(abort: AbortController) {
    this.#abort = abort;
  }

  /** @internal — minted by `Client.laboratoriesListListener`. */
  static async _connect(mode: ClientMode): Promise<LaboratoriesListListener> {
    const abort = new AbortController();
    const events = await openStream(mode, abort.signal, {
      route: "/laboratories/list",
      viewerCommand: "daemon_laboratories_list",
    });
    const listener = new LaboratoriesListListener(abort);
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

  /** Snapshot the current laboratory set, sorted by `id`. */
  laboratories(): LaboratoryStatus[] {
    return [...this.#state.values()].sort((a, b) =>
      a.id < b.id ? -1 : a.id > b.id ? 1 : 0,
    );
  }

  /** Resolves on the next change applied to the state. A fresh call
   * waits for the FIRST change after it is made — loop with the
   * {@link laboratories} read. Resolves immediately if already
   * closed. */
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

  /** Wake every {@link subscribe} waiter. */
  #notify(): void {
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
