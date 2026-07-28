/**
 * Materialized consumer of the cli daemon's `/agents/instances/list`
 * endpoint — the JS mirror of the Rust SDK's
 * `daemon::agents_instances_list_listener::AgentsInstancesListListener`,
 * identical in semantics.
 *
 * Deliberately minimal wire: each item is an AIH plus its live
 * `active` flag — nothing else. Per-agent detail (tags, spawn /
 * last-active timestamps, counters) lives on the per-agent
 * `/agents/instances/{*aih}` endpoint. The listener folds events into
 * a self-updating `AIH → active` map: a `snapshot` replaces the whole
 * set, `activated` upserts an AIH to active (introducing it if
 * unseen), `deactivated` flips it to inactive in place (kept — the
 * endpoint lists all known agents). Activity itself is lock-driven on
 * the daemon (kernel-released on holder death), so a spawn killed
 * mid-stream flips to inactive exactly.
 *
 * One listener = one connection: when the socket closes the view
 * freezes at its last state; reconnect is the caller's loop — mint a
 * new listener from the client. Unparseable events are skipped
 * (forward compat). No runtime validation, like
 * {@link BroadcastListener}.
 */

import type {
  DaemonAgentsInstancesListListenerAgentEvent,
  DaemonAgentsInstancesListListenerAgentStatus,
} from "./agents_instances_list_listener";
import { openStream, type ClientMode } from "./mode";

type AgentStatus = DaemonAgentsInstancesListListenerAgentStatus;
type AgentEvent = DaemonAgentsInstancesListListenerAgentEvent;

/**
 * The materialized `/agents/instances/list` view — minted by
 * {@link Client.agentsInstancesListListener}; resolves only once the
 * stream has OPENED.
 *
 * ```ts
 * const listener = await client.agentsInstancesListListener();
 * listener.agents(); // [{ agent_instance_hierarchy, active }, ...]
 * await listener.subscribe(); // resolves on the next change
 * ```
 */
export class AgentsInstancesListListener {
  #abort: AbortController;
  #closed = false;
  /** `AIH → active`. */
  #state = new Map<string, boolean>();
  /** Resolvers for the in-flight {@link subscribe} promises. */
  #waiters = new Set<() => void>();

  private constructor(abort: AbortController) {
    this.#abort = abort;
  }

  /** @internal — minted by `Client.agentsInstancesListListener`. */
  static async _connect(mode: ClientMode): Promise<AgentsInstancesListListener> {
    const abort = new AbortController();
    const events = await openStream(mode, abort.signal, {
      route: "/agents/instances/list",
      viewerCommand: "daemon_agents_instances_list",
    });
    const listener = new AgentsInstancesListListener(abort);
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

  /** Snapshot the current agent set, sorted by `agent_instance_hierarchy`. */
  agents(): AgentStatus[] {
    return [...this.#state.entries()]
      .sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))
      .map(([agent_instance_hierarchy, active]) => ({
        agent_instance_hierarchy,
        active,
      }));
  }

  /** Resolves on the next change applied to the state. A fresh call
   * waits for the FIRST change after it is made — loop with the
   * {@link agents} read. Resolves immediately if already closed. */
  subscribe(): Promise<void> {
    if (this.#closed) return Promise.resolve();
    return new Promise<void>((resolve) => {
      this.#waiters.add(resolve);
    });
  }

  #onFrame(frame: unknown): void {
    if (typeof frame !== "object" || frame === null) return;
    if (typeof (frame as { type?: unknown }).type !== "string") return;
    if (this.#apply(frame as AgentEvent)) {
      this.#notify();
    }
  }

  /** Fold one event into the map. `true` when a known event was
   * applied (an unrecognized `type` is a no-op — no spurious change). */
  #apply(event: AgentEvent): boolean {
    switch (event.type) {
      case "snapshot":
        this.#state.clear();
        for (const agent of event.agents) {
          this.#state.set(agent.agent_instance_hierarchy, agent.active);
        }
        return true;
      case "activated":
        this.#state.set(event.agent_instance_hierarchy, true);
        return true;
      case "deactivated":
        this.#state.set(event.agent_instance_hierarchy, false);
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
