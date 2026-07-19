/**
 * Materialized consumer of the cli daemon's `/channels` endpoint — the
 * DUPLEX CHANNELS surface — over Server-Sent Events (SSE): the JS
 * mirror of the Rust SDK's `cli::channel_listener::ChannelListener`.
 *
 * A publisher offers a channel (`channels publish`); the first
 * connected client to ACCEPT owns it. This listener folds every
 * incoming event:
 * - `connection` captures this connection's secret (`S_conn`), needed
 *   to accept offers — it is always the FIRST frame.
 * - `offer` inserts into the pending-offers map; `offer_withdrawn` and
 *   `closed` remove.
 * - `owner_secret` records the channel's owner secret (`S_owner`) and
 *   fulfils any pending {@link ChannelListener.accept}.
 * - `live` is the caught-up marker (sent once, after the offer replay).
 *
 * Accepting is {@link ChannelListener.accept}: it POSTs the stored
 * `S_conn` to `/channels/{id}/accept`, then awaits the owner secret the
 * daemon pushes back over THIS stream (never in the POST response) —
 * so the caller must be the process holding the connection. The
 * returned `S_owner` authorizes `channels logs reply|list|open|
 * subscribe` for that channel.
 *
 * Auth rides the `X-OBJECTIVEAI-SIGNATURE` request header. One listener
 * = one connection: when the socket closes the view freezes; reconnect
 * is the caller's loop. Fetch mode only for now (a viewer proxy is a
 * later migration). Unparseable events are skipped (forward compat).
 */

import { connectSse } from "./sse";

import type {
  CliChannelListenerChannelAcceptOutcome,
  CliChannelListenerChannelEvent,
  CliChannelListenerChannelOffer,
} from "./channel_listener";

type ChannelEvent = CliChannelListenerChannelEvent;
type ChannelOffer = CliChannelListenerChannelOffer;
type ChannelAcceptOutcome = CliChannelListenerChannelAcceptOutcome;

export interface ChannelListenerOptions {
  /** The pre-derived `sha256=<hex(SHA256(DAEMON_SECRET))>`, sent as the
   * `X-OBJECTIVEAI-SIGNATURE` header. Without it the daemon must be
   * running without a secret. */
  signature?: string | null;
  /** Invoked with every parsed event after it is folded — including
   * the `connection` secret, offer withdrawals, closes, and the `live`
   * caught-up marker. Runs synchronously on frame receipt; keep it
   * cheap. */
  onEvent?: (event: ChannelEvent) => void;
}

/**
 * The materialized `/channels` view + accept client — construct via
 * {@link ChannelListener.connect}.
 *
 * ```ts
 * const owner = await ChannelListener.connect(
 *   "http://127.0.0.1:49152",
 *   { signature },
 * );
 * // ...a publisher runs `channels publish --key demo --details '{}'`...
 * await owner.subscribe();                 // wakes on the offer
 * const [offer] = owner.pending();
 * const sOwner = await owner.accept(offer.channel_id); // returns S_owner
 * // ...then use S_owner with `channels logs reply|list|open|subscribe`.
 * ```
 */
export class ChannelListener {
  #abort: AbortController;
  #closed = false;
  #baseUrl: string;
  #signature?: string | null;
  /** This connection's secret (`S_conn`), from the first frame. */
  #connSecret: string | null = null;
  /** `channel_id → offer` — the currently OPEN offers. */
  #offers = new Map<string, ChannelOffer>();
  /** `channel_id → S_owner` — owner secrets for accepted channels. */
  #ownerSecrets = new Map<string, string>();
  /** In-flight {@link accept} calls awaiting their owner secret. */
  #acceptWaiters = new Map<
    string,
    { resolve: (secret: string) => void; reject: (err: Error) => void }
  >();
  #onEvent?: (event: ChannelEvent) => void;
  /** Resolvers for the in-flight {@link subscribe} promises. */
  #waiters = new Set<() => void>();

  private constructor(
    abort: AbortController,
    baseUrl: string,
    signature?: string | null,
    onEvent?: (event: ChannelEvent) => void,
  ) {
    this.#abort = abort;
    this.#baseUrl = baseUrl;
    this.#signature = signature;
    this.#onEvent = onEvent;
  }

  /**
   * Open the connection and resolve once the socket is established.
   * `baseUrl` is the daemon's published base address (e.g.
   * `http://127.0.0.1:49152`) — `/channels` and `/channels/{id}/accept`
   * are appended. The returned listener immediately begins folding
   * events (the connection secret first, then the open-offer replay,
   * then the `live` marker).
   */
  static async connect(
    baseUrl: string,
    options?: ChannelListenerOptions,
  ): Promise<ChannelListener> {
    const base = baseUrl.replace(/\/+$/, "");
    const abort = new AbortController();
    let events: AsyncGenerator<string>;
    try {
      events = await connectSse(
        `${base}/channels`,
        options?.signature,
        abort.signal,
      );
    } catch (e) {
      throw new Error(`connect daemon channels sse: ${String(e)}`);
    }
    const listener = new ChannelListener(
      abort,
      base,
      options?.signature,
      options?.onEvent,
    );
    listener.#pump(events);
    return listener;
  }

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
   * {@link subscribe} / {@link accept} resolves/rejects. */
  close(): void {
    if (this.#closed) return;
    this.#abort.abort();
    this.#onClose();
  }

  /** This connection's secret (`S_conn`), once the first frame has
   * arrived. */
  connectionSecret(): string | null {
    return this.#connSecret;
  }

  /** Snapshot the currently open offers, sorted by `channel_id`. */
  pending(): ChannelOffer[] {
    return [...this.#offers.values()].sort((a, b) =>
      a.channel_id < b.channel_id ? -1 : a.channel_id > b.channel_id ? 1 : 0,
    );
  }

  /** The owner secret (`S_owner`) for a channel this connection has
   * accepted, if known. */
  ownerSecret(channelId: string): string | null {
    return this.#ownerSecrets.get(channelId) ?? null;
  }

  /** Resolves on the next applied event. Resolves immediately if
   * already closed. */
  subscribe(): Promise<void> {
    if (this.#closed) return Promise.resolve();
    return new Promise<void>((resolve) => {
      this.#waiters.add(resolve);
    });
  }

  /**
   * Accept an open offer: `POST /channels/{id}/accept` with this
   * connection's `S_conn`, then await the owner secret (`S_owner`) the
   * daemon pushes back over the SSE. Returns `S_owner`.
   *
   * Rejects if the connection secret hasn't arrived yet, the daemon
   * refuses the accept (a non-`accepted` outcome), or the socket closes
   * before the secret is delivered.
   */
  async accept(channelId: string): Promise<string> {
    if (this.#connSecret === null) {
      throw new Error("channel listener not connected: no connection secret yet");
    }
    // Register the waiter BEFORE the POST so a fast owner_secret frame
    // is never missed. A close rejects it (see #onClose).
    const secretPromise = new Promise<string>((resolve, reject) => {
      this.#acceptWaiters.set(channelId, { resolve, reject });
    });
    const headers: Record<string, string> = {
      "Content-Type": "application/json",
    };
    if (this.#signature) {
      headers["X-OBJECTIVEAI-SIGNATURE"] = this.#signature;
    }
    let response: Response;
    try {
      response = await fetch(
        `${this.#baseUrl}/channels/${encodeURIComponent(channelId)}/accept`,
        {
          method: "POST",
          headers,
          body: JSON.stringify({ conn_secret: this.#connSecret }),
        },
      );
    } catch (e) {
      this.#acceptWaiters.delete(channelId);
      throw new Error(`channel accept: fetch failed: ${String(e)}`);
    }
    const body = await response.text();
    let outcome: ChannelAcceptOutcome;
    try {
      outcome = JSON.parse(body) as ChannelAcceptOutcome;
    } catch {
      this.#acceptWaiters.delete(channelId);
      throw new Error(
        `channel accept: HTTP ${response.status} with unparseable body: ${body}`,
      );
    }
    if (outcome.type !== "accepted") {
      this.#acceptWaiters.delete(channelId);
      throw new Error(`channel accept refused: ${outcome.type}`);
    }
    // The secret may already have landed over the SSE.
    const already = this.#ownerSecrets.get(channelId);
    if (already !== undefined) {
      this.#acceptWaiters.delete(channelId);
      return already;
    }
    return secretPromise;
  }

  #onFrame(frame: unknown): void {
    if (typeof frame !== "object" || frame === null) return;
    if (typeof (frame as { type?: unknown }).type !== "string") return;
    if (this.#apply(frame as ChannelEvent)) {
      this.#notify(frame as ChannelEvent);
    }
  }

  /** Fold one event into the shared state. `true` when a known event
   * was applied. */
  #apply(event: ChannelEvent): boolean {
    switch (event.type) {
      case "connection":
        this.#connSecret = event.secret;
        return true;
      case "offer":
        this.#offers.set(event.offer.channel_id, event.offer);
        return true;
      case "offer_withdrawn":
      case "closed":
        this.#offers.delete(event.channel_id);
        return true;
      case "owner_secret": {
        this.#ownerSecrets.set(event.channel_id, event.secret);
        const waiter = this.#acceptWaiters.get(event.channel_id);
        if (waiter) {
          this.#acceptWaiters.delete(event.channel_id);
          waiter.resolve(event.secret);
        }
        return true;
      }
      case "live":
        return true;
      default:
        return false;
    }
  }

  #notify(event: ChannelEvent): void {
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
    const accepters = [...this.#acceptWaiters.values()];
    this.#acceptWaiters.clear();
    for (const w of accepters) {
      w.reject(new Error("channel listener closed before owner secret"));
    }
    const waiters = [...this.#waiters];
    this.#waiters.clear();
    for (const wake of waiters) wake();
  }
}
