/**
 * Materialized consumer of the cli daemon's `/agents/instances/list`
 * endpoint over Server-Sent Events (SSE) — the JS mirror of the Rust SDK's
 * `cli::websocket_agents_instances_list_listener::WebSocketAgentsInstancesListListener`,
 * identical in construction and semantics.
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
 * Auth rides the `X-OBJECTIVEAI-SIGNATURE` request header. One listener = one connection: when the socket closes the
 * view freezes at its last state; reconnection is the caller's loop.
 * Unparseable events are skipped (forward compat). No runtime
 * validation, like {@link WebSocketListener}.
 */

import { connectSse } from "./sse";

import type {
  CliWebsocketAgentsInstancesListListenerAgentEvent,
  CliWebsocketAgentsInstancesListListenerAgentStatus,
} from "./websocket_agents_instances_list_listener";

type AgentStatus = CliWebsocketAgentsInstancesListListenerAgentStatus;
type AgentEvent = CliWebsocketAgentsInstancesListListenerAgentEvent;

export interface WebSocketAgentsInstancesListListenerOptions {
  /** The pre-derived `sha256=<hex(SHA256(DAEMON_SECRET))>`, sent
   * as the `X-OBJECTIVEAI-SIGNATURE` header. Without it the daemon must be
   * running without a secret. */
  signature?: string | null;
  /** Invoked with the full current agent set (sorted by AIH) after
   * every applied change. Runs synchronously on frame receipt, so keep
   * it cheap; for the full state on demand use
   * {@link WebSocketAgentsInstancesListListener.agents}. */
  onChange?: (agents: AgentStatus[]) => void;
}

/**
 * The materialized `/agents/instances/list` view — construct via
 * {@link WebSocketAgentsInstancesListListener.connect}.
 *
 * ```ts
 * const listener = await WebSocketAgentsInstancesListListener.connect(
 *   "ws://127.0.0.1:49152/agents/instances/list",
 *   { signature, onChange: (agents) => render(agents) },
 * );
 * listener.agents(); // [{ agent_instance_hierarchy, active }, ...]
 * await listener.subscribe(); // resolves on the next change
 * ```
 */
export class WebSocketAgentsInstancesListListener {
  #abort: AbortController;
  #closed = false;
  /** `AIH → active`. */
  #state = new Map<string, boolean>();
  #onChange?: (agents: AgentStatus[]) => void;
  /** Resolvers for the in-flight {@link subscribe} promises. */
  #waiters = new Set<() => void>();

  private constructor(
    abort: AbortController,
    onChange?: (agents: AgentStatus[]) => void,
  ) {
    this.#abort = abort;
    this.#onChange = onChange;
  }

  /**
   * Open the connection, send the auth preamble, and resolve once the
   * socket is established (rejects when the daemon is unreachable or
   * closes during the handshake — e.g. it refused the auth). `url` is
   * the daemon's full `/agents/instances/list` URL. The returned
   * listener immediately begins folding events (the first is the
   * connect-time snapshot).
   */
  static async connect(
    url: string,
    options?: WebSocketAgentsInstancesListListenerOptions,
  ): Promise<WebSocketAgentsInstancesListListener> {
    const abort = new AbortController();
    let events: AsyncGenerator<string>;
    try {
      events = await connectSse(url, options?.signature, abort.signal);
    } catch (e) {
      throw new Error(`connect daemon agents sse: ${String(e)}`);
    }
    const listener = new WebSocketAgentsInstancesListListener(
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
   * {@link agents} read, or use
   * {@link WebSocketAgentsInstancesListListenerOptions.onChange} for
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

  /** Fire the callback with the refreshed set and wake every
   * {@link subscribe} waiter. */
  #notify(): void {
    if (this.#onChange) {
      this.#onChange(this.agents());
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
