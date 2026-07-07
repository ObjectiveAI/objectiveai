/**
 * Materialized consumer of the cli daemon's `/agents/instances/list`
 * endpoint over a NATIVE WebSocket — the JS mirror of the Rust SDK's
 * `cli::websocket_agents_instances_list_listener::WebSocketAgentsInstancesListListener`, identical in
 * construction and semantics.
 *
 * It is NOT a raw event stream: it connects once, then folds every
 * incoming `AgentEvent` into an in-memory, self-updating view of the
 * current agent set (keyed by `agent_instance_hierarchy`). A `snapshot`
 * replaces the whole set, `activated` / `updated` upsert one agent, and
 * `deactivated` flips one to inactive (and stamps its `last_active_at`).
 * The record is KEPT on deactivation — the view mirrors the endpoint's
 * "all agents" semantics.
 *
 * Three ways to observe it, matching the Rust API:
 * - {@link WebSocketAgentsInstancesListListener.agents} — snapshot of the current set
 *   (sorted by AIH).
 * - an on-change **callback** ({@link WebSocketAgentsInstancesListListenerOptions.onChange}),
 *   invoked with the full refreshed set on every applied change.
 * - {@link WebSocketAgentsInstancesListListener.subscribe} — a Promise that resolves on
 *   the next change.
 *
 * Auth is the first-message preamble shared by every daemon WebSocket
 * route: the connection's first outbound text frame is
 * `{"signature": "sha256=<hex>"}` (or `{"signature": null}`); a
 * secret-bearing daemon closes the connection on a missing/invalid
 * signature.
 *
 * One listener = one connection: the state updates until the socket
 * closes; after that the view is frozen at its last state and any pending
 * {@link WebSocketAgentsInstancesListListener.subscribe} resolves. Reconnection (the
 * daemon's address changes across restarts) is the caller's loop — build a
 * new listener. No runtime validation: frame bodies are structural claims
 * over the wire, like {@link WebSocketListener}.
 */

import type {
  CliWebsocketAgentsInstancesListListenerAgentEvent,
  CliWebsocketAgentsInstancesListListenerAgentRecord,
} from "./websocket_agents_instances_list_listener";

type AgentRecord = CliWebsocketAgentsInstancesListListenerAgentRecord;
type AgentEvent = CliWebsocketAgentsInstancesListListenerAgentEvent;

export interface WebSocketAgentsInstancesListListenerOptions {
  /** The pre-derived `sha256=<hex(SHA256(DAEMON_SECRET))>`, sent verbatim
   * in the auth preamble. Without it the daemon must be running without a
   * secret. */
  signature?: string | null;
  /** Invoked with the full current agent set (sorted by AIH) after every
   * applied change. Runs synchronously on frame receipt, so keep it cheap;
   * for the full state on demand use {@link WebSocketAgentsInstancesListListener.agents}. */
  onChange?: (agents: AgentRecord[]) => void;
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
 * // ...or pull on demand / wait for changes:
 * const current = listener.agents();
 * await listener.subscribe(); // resolves on the next change
 * ```
 */
export class WebSocketAgentsInstancesListListener {
  #ws: WebSocket;
  #closed = false;
  /** Current set, keyed by `agent_instance_hierarchy`. */
  #state = new Map<string, AgentRecord>();
  #onChange?: (agents: AgentRecord[]) => void;
  /** Resolvers for the in-flight {@link subscribe} promises. */
  #waiters = new Set<() => void>();

  private constructor(
    ws: WebSocket,
    onChange?: (agents: AgentRecord[]) => void,
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
   * socket is established (rejects when the daemon is unreachable or closes
   * during the handshake — e.g. it refused the auth). `url` is the
   * daemon's full `/agents/instances/list` URL. The returned listener
   * immediately begins folding events (the first is the connect-time
   * snapshot).
   */
  static connect(
    url: string,
    options?: WebSocketAgentsInstancesListListenerOptions,
  ): Promise<WebSocketAgentsInstancesListListener> {
    return new Promise((resolve, reject) => {
      let ws: WebSocket;
      try {
        ws = new WebSocket(url);
      } catch (e) {
        reject(new Error(`connect daemon agents websocket: ${String(e)}`));
        return;
      }
      ws.onopen = () => {
        // The auth preamble — always the connection's first text frame,
        // `{"signature": null}` against a secretless daemon.
        ws.send(JSON.stringify({ signature: options?.signature ?? null }));
        resolve(new WebSocketAgentsInstancesListListener(ws, options?.onChange));
      };
      ws.onclose = (event: CloseEvent) => {
        reject(
          new Error(
            `connect daemon agents websocket: connection closed` +
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

  /** Snapshot the current agent set, sorted by `agent_instance_hierarchy`.
   * Element identities are stable across calls until an agent changes. */
  agents(): AgentRecord[] {
    return [...this.#state.values()].sort((a, b) =>
      a.agent_instance_hierarchy < b.agent_instance_hierarchy
        ? -1
        : a.agent_instance_hierarchy > b.agent_instance_hierarchy
          ? 1
          : 0,
    );
  }

  /** Resolves on the next change applied to the state. A fresh call waits
   * for the FIRST change after it is made, so a change between a preceding
   * {@link agents} read and this call is not observed by it — loop with the
   * read, or use {@link WebSocketAgentsInstancesListListenerOptions.onChange} for
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

  /** Fold one event into the current set. `true` when a known event was
   * applied (an unrecognized `type` is a no-op — no spurious change). */
  #apply(event: AgentEvent): boolean {
    switch (event.type) {
      case "snapshot":
        this.#state.clear();
        for (const agent of event.agents) {
          this.#state.set(agent.agent_instance_hierarchy, agent);
        }
        return true;
      case "activated":
      case "updated":
        this.#state.set(event.agent.agent_instance_hierarchy, event.agent);
        return true;
      case "deactivated": {
        const record = this.#state.get(event.agent_instance_hierarchy);
        if (record) {
          // Replace (not mutate) so consumers holding a prior reference
          // keep their snapshot.
          this.#state.set(event.agent_instance_hierarchy, {
            ...record,
            active: false,
            last_active_at: event.last_active_at ?? null,
          });
        }
        return true;
      }
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
