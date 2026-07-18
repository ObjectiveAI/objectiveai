/**
 * Materialized consumer of the cli daemon's `/user` endpoint — the
 * USER REQUESTS channel — over Server-Sent Events (SSE): the JS
 * mirror of the Rust SDK's `cli::user_listener::UserListener`,
 * identical in construction and semantics.
 *
 * Plugins (or any command caller) broadcast an outbound request to
 * every connected user stream; the FIRST ACCEPTED reply wins and
 * unblocks the originating `user request` command. The listener folds
 * events into a self-updating `id → request` map of the CURRENTLY
 * PENDING requests: a `request` event (live broadcast or connect-time
 * replay) inserts, `settled`/`timed_out` remove, and `live` (the
 * caught-up marker sent once per connection, right after the replay)
 * leaves the state untouched while still reaching {@link
 * UserListenerOptions.onEvent} and waking {@link
 * UserListener.subscribe} waiters. A settled request is never
 * replayed to new connections.
 *
 * Replying is {@link UserListener.reply}: `POST /user/{id}/reply`.
 * The daemon arbitrates — exactly one reply is ever accepted (first
 * wins) — and every outcome comes back as a typed {@link
 * CliUserListenerUserReplyOutcome} whatever the HTTP status.
 *
 * Auth rides the `X-OBJECTIVEAI-SIGNATURE` request header (fetch
 * mode). One listener = one connection: when the socket closes the
 * view freezes at its last state; reconnection is the caller's loop.
 * Unparseable events are skipped (forward compat). No runtime
 * validation, like {@link BroadcastListener}.
 */

import { connectSse } from "./sse";
import { connectViewerStream, type ViewerTransport } from "./viewer";

import type { CliCommandAgentArguments } from "./command";
import type {
  CliUserListenerUserEvent,
  CliUserListenerUserReplyOutcome,
  CliUserListenerUserRequest,
} from "./user_listener";

type UserRequest = CliUserListenerUserRequest;
type UserEvent = CliUserListenerUserEvent;
type UserReplyOutcome = CliUserListenerUserReplyOutcome;

export interface UserListenerOptions {
  /** The pre-derived `sha256=<hex(SHA256(DAEMON_SECRET))>`, sent
   * as the `X-OBJECTIVEAI-SIGNATURE` header. Without it the daemon must be
   * running without a secret. */
  signature?: string | null;
  /** Invoked with every parsed event after it is folded — including
   * `settled`/`timed_out` edges and the `live` caught-up marker (a UI
   * needs the edges, not just the folded set). Runs synchronously on
   * frame receipt, so keep it cheap; for the pending state on demand
   * use {@link UserListener.pending}. */
  onEvent?: (event: UserEvent) => void;
}

/** {@link UserListener.connectViewer} options — the regular options
 * sans `signature` (the Rust proxy stamps auth). */
export type UserListenerViewerOptions = Omit<UserListenerOptions, "signature">;

/** How {@link UserListener.reply} reaches the daemon: a direct fetch
 * against the base URL (fetch mode) or the Tauri IPC proxy command
 * (viewer mode — the Rust side owns address, signature, and the
 * viewer identity). */
type ReplyTransport =
  | { kind: "fetch"; baseUrl: string; signature?: string | null }
  | { kind: "viewer"; transport: ViewerTransport };

/**
 * The materialized `/user` view + reply client — construct via
 * {@link UserListener.connect} (direct fetch) or
 * {@link UserListener.connectViewer} (Tauri proxy).
 *
 * ```ts
 * const listener = await UserListener.connect(
 *   "http://127.0.0.1:49152",
 *   { signature, onEvent: (event) => render(event) },
 * );
 * listener.pending(); // [{ id, key, details, agent_arguments, ... }, ...]
 * await listener.subscribe(); // resolves on the next event
 * await listener.reply(id, "yes", {
 *   agent_instance_hierarchy: "human",
 * }); // { type: "accepted" } | { type: "rejected", message } | ...
 * ```
 */
export class UserListener {
  #abort: AbortController;
  #closed = false;
  /** `id → request` — the currently PENDING requests. */
  #state = new Map<string, UserRequest>();
  #onEvent?: (event: UserEvent) => void;
  /** Resolvers for the in-flight {@link subscribe} promises. */
  #waiters = new Set<() => void>();
  #reply: ReplyTransport;

  private constructor(
    abort: AbortController,
    reply: ReplyTransport,
    onEvent?: (event: UserEvent) => void,
  ) {
    this.#abort = abort;
    this.#reply = reply;
    this.#onEvent = onEvent;
  }

  /**
   * Open the connection and resolve once the socket is established
   * (rejects when the daemon is unreachable or refused the auth).
   * `baseUrl` is the daemon's published base address (e.g.
   * `http://127.0.0.1:49152`) — `/user` and `/user/{id}/reply` are
   * appended. The returned listener immediately begins folding events
   * (the connect-time replay of every pending request first, then the
   * `live` caught-up marker).
   */
  static async connect(
    baseUrl: string,
    options?: UserListenerOptions,
  ): Promise<UserListener> {
    const base = baseUrl.replace(/\/+$/, "");
    const abort = new AbortController();
    let events: AsyncGenerator<string>;
    try {
      events = await connectSse(`${base}/user`, options?.signature, abort.signal);
    } catch (e) {
      throw new Error(`connect daemon user sse: ${String(e)}`);
    }
    const listener = new UserListener(
      abort,
      { kind: "fetch", baseUrl: base, signature: options?.signature },
      options?.onEvent,
    );
    listener.#pump(events);
    return listener;
  }

  /**
   * Viewer-mode connect: the stream rides the Tauri IPC proxy
   * ({@link connectViewerStream}) instead of fetch — no address, no
   * signature, no identity (the Rust side owns all three; replies
   * carry the viewer's identity). Same resolve/reject and lifecycle
   * semantics as {@link connect}; reconnection remains the caller's
   * loop.
   */
  static async connectViewer(
    transport: ViewerTransport,
    options?: UserListenerViewerOptions,
  ): Promise<UserListener> {
    const abort = new AbortController();
    let events: AsyncGenerator<string>;
    try {
      events = await connectViewerStream(
        transport,
        "daemon_user",
        {},
        abort.signal,
      );
    } catch (e) {
      throw new Error(`connect daemon user sse: ${String(e)}`);
    }
    const listener = new UserListener(
      abort,
      { kind: "viewer", transport },
      options?.onEvent,
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

  /** Snapshot the currently pending requests, sorted by `id`. */
  pending(): UserRequest[] {
    return [...this.#state.values()].sort((a, b) =>
      a.id < b.id ? -1 : a.id > b.id ? 1 : 0,
    );
  }

  /** Resolves on the next applied event. A fresh call waits for the
   * FIRST event after it is made — loop with the {@link pending}
   * read, or use {@link UserListenerOptions.onEvent} for guaranteed
   * push. Resolves immediately if already closed. */
  subscribe(): Promise<void> {
    if (this.#closed) return Promise.resolve();
    return new Promise<void>((resolve) => {
      this.#waiters.add(resolve);
    });
  }

  /**
   * Reply to one pending request. The daemon answers with a typed
   * outcome whatever the HTTP status: `accepted` (this reply WON),
   * `settled` (another reply already won), or `not_found`.
   *
   * `identity` (the replier's `X-OBJECTIVEAI-*` headers) applies in
   * FETCH mode only; in viewer mode the Rust proxy stamps the
   * viewer's own identity and this argument is ignored.
   */
  async reply(
    id: string,
    reply: unknown,
    identity?: CliCommandAgentArguments,
  ): Promise<UserReplyOutcome> {
    if (this.#reply.kind === "viewer") {
      return (await this.#reply.transport.invoke("daemon_user_reply", {
        id,
        reply,
      })) as UserReplyOutcome;
    }
    const headers: Record<string, string> = {
      "Content-Type": "application/json",
    };
    if (this.#reply.signature) {
      headers["X-OBJECTIVEAI-SIGNATURE"] = this.#reply.signature;
    }
    if (identity?.agent_instance_hierarchy) {
      headers["X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY"] =
        identity.agent_instance_hierarchy;
    }
    if (identity?.agent_id) {
      headers["X-OBJECTIVEAI-AGENT-ID"] = identity.agent_id;
    }
    if (identity?.agent_full_id) {
      headers["X-OBJECTIVEAI-AGENT-FULL-ID"] = identity.agent_full_id;
    }
    if (identity?.agent_remote) {
      headers["X-OBJECTIVEAI-AGENT-REMOTE"] = identity.agent_remote;
    }
    if (identity?.response_id) {
      headers["X-OBJECTIVEAI-RESPONSE-ID"] = identity.response_id;
    }
    if (identity?.response_ids) {
      headers["X-OBJECTIVEAI-RESPONSE-IDS"] = identity.response_ids;
    }
    let response: Response;
    try {
      response = await fetch(
        `${this.#reply.baseUrl}/user/${encodeURIComponent(id)}/reply`,
        {
          method: "POST",
          headers,
          body: JSON.stringify({ reply }),
        },
      );
    } catch (e) {
      throw new Error(`user reply: fetch failed: ${String(e)}`);
    }
    const body = await response.text();
    try {
      return JSON.parse(body) as UserReplyOutcome;
    } catch {
      throw new Error(
        `user reply: HTTP ${response.status} with unparseable body: ${body}`,
      );
    }
  }

  #onFrame(frame: unknown): void {
    if (typeof frame !== "object" || frame === null) return;
    if (typeof (frame as { type?: unknown }).type !== "string") return;
    if (this.#apply(frame as UserEvent)) {
      this.#notify(frame as UserEvent);
    }
  }

  /** Fold one event into the pending map. `true` when a known event
   * was applied (an unrecognized `type` is a no-op — no spurious
   * change). `live` applies without touching the state — it is the
   * caught-up marker, and consumers await it to prove absence. */
  #apply(event: UserEvent): boolean {
    switch (event.type) {
      case "request":
        this.#state.set(event.request.id, event.request);
        return true;
      case "settled":
      case "timed_out":
        this.#state.delete(event.id);
        return true;
      case "live":
        return true;
      default:
        return false;
    }
  }

  /** Fire the callback with the applied event and wake every
   * {@link subscribe} waiter. */
  #notify(event: UserEvent): void {
    if (this.#onEvent) {
      this.#onEvent(event);
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
