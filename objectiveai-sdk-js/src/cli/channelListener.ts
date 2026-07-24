/**
 * Materialized consumer of the cli daemon's `/channels` endpoints — the
 * DUPLEX CHANNELS surface — over Server-Sent Events (SSE): the JS
 * mirror of the Rust SDK's `cli::channel_listener::ChannelListener`.
 *
 * `GET /channels` is the OFFER lifecycle stream and nothing else: the
 * listener folds `offer` (insert into the pending map),
 * `offer_withdrawn` (remove), and the `live` caught-up marker. No
 * secrets ride this stream.
 *
 * Accepting is {@link ChannelListener.accept}: a bare
 * `POST /channels/{id}/accept` (first-wins) whose `200` body carries
 * the owner secret (`S_owner`) — the per-channel capability for
 * `channels logs reply|list|open|subscribe` and `channels close`. The
 * daemon tracks no liveness; a channel stays open until someone runs
 * `channels close` with either of its secrets (terminal — any blocked
 * `channels logs subscribe` unblocks with `channel_closed`).
 *
 * Auth rides the `X-OBJECTIVEAI-SIGNATURE` request header — in FETCH
 * mode only. A viewer-proxy migration must NOT set it (nor take a
 * signature at all): in viewer mode the Rust proxy stamps auth and
 * identity headers natively, exactly as it does for agent arguments —
 * secrets never enter the webview. One listener = one connection:
 * when the socket closes the view freezes; reconnect is the caller's
 * loop. Fetch mode only for now. Unparseable events are skipped
 * (forward compat).
 */

import { connectSse } from "./sse";

import type {
  CliChannelListenerChannelEvent,
  CliChannelListenerChannelOffer,
} from "./channel_listener";

type ChannelEvent = CliChannelListenerChannelEvent;
type ChannelOffer = CliChannelListenerChannelOffer;

export interface ChannelListenerOptions {
  /** The pre-derived `sha256=<hex(SHA256(DAEMON_SECRET))>`, sent as the
   * `X-OBJECTIVEAI-SIGNATURE` header. Without it the daemon must be
   * running without a secret. */
  signature?: string | null;
  /** Invoked with every parsed event after it is folded — offers,
   * withdrawals, and the `live` caught-up marker. Runs synchronously
   * on frame receipt; keep it cheap. */
  onEvent?: (event: ChannelEvent) => void;
}

/**
 * The materialized `/channels` offer view + accept client — construct
 * via {@link ChannelListener.connect}.
 *
 * ```ts
 * const surface = await ChannelListener.connect(
 *   "http://127.0.0.1:49152",
 *   { signature },
 * );
 * // ...a publisher runs `channels publish --key demo --details '{}'`...
 * await surface.subscribe();               // wakes on the offer
 * const [offer] = surface.pending();
 * const sOwner = await surface.accept(offer.channel_id);
 * // ...use S_owner with `channels logs reply|list|open|subscribe`,
 * // and `channels close` when the conversation is over.
 * ```
 */
export class ChannelListener {
  #abort: AbortController;
  #closed = false;
  #baseUrl: string;
  #signature?: string | null;
  /** `channel_id → offer` — the currently OPEN offers. */
  #offers = new Map<string, ChannelOffer>();
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
   * events (the open-offer replay, then the `live` marker).
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
   * {@link subscribe} resolves. */
  close(): void {
    if (this.#closed) return;
    this.#abort.abort();
    this.#onClose();
  }

  /** Snapshot the currently open offers, sorted by `channel_id`. */
  pending(): ChannelOffer[] {
    return [...this.#offers.values()].sort((a, b) =>
      a.channel_id < b.channel_id ? -1 : a.channel_id > b.channel_id ? 1 : 0,
    );
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
   * Accept an open offer: a bare `POST /channels/{id}/accept`
   * (first-wins). Resolves with the owner secret (`S_owner`) from the
   * response body. Rejects on refusal — the message carries the HTTP
   * status (404 unknown/withdrawn, 409 already accepted, 401
   * unauthorized).
   */
  async accept(channelId: string): Promise<string> {
    const headers: Record<string, string> = {};
    if (this.#signature) {
      headers["X-OBJECTIVEAI-SIGNATURE"] = this.#signature;
    }
    let response: Response;
    try {
      response = await fetch(
        `${this.#baseUrl}/channels/${encodeURIComponent(channelId)}/accept`,
        { method: "POST", headers },
      );
    } catch (e) {
      throw new Error(`channel accept: fetch failed: ${String(e)}`);
    }
    if (!response.ok) {
      throw new Error(`channel accept: HTTP ${response.status}`);
    }
    const body = await response.text();
    // TODO: switch to the generated ChannelAccepted zod mirror once
    // the json-schema regen lands.
    let accepted: { secret?: unknown };
    try {
      accepted = JSON.parse(body) as { secret?: unknown };
    } catch {
      throw new Error(`channel accept: unparseable body: ${body}`);
    }
    if (typeof accepted.secret !== "string") {
      throw new Error(`channel accept: body carried no secret: ${body}`);
    }
    return accepted.secret;
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
      case "offer":
        this.#offers.set(event.offer.channel_id, event.offer);
        return true;
      case "offer_withdrawn":
        this.#offers.delete(event.channel_id);
        return true;
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
    const waiters = [...this.#waiters];
    this.#waiters.clear();
    for (const wake of waiters) wake();
  }
}
