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
 * Accepting is {@link ChannelListener.accept}: it OPENS the offer's
 * per-channel stream (`GET /channels/{id}`) — the open IS the accept
 * (first-wins). The stream's first frame delivers the owner secret
 * (`S_owner`), and the returned {@link ChannelStream} is the channel's
 * LIVENESS ANCHOR: closing it closes the channel (terminal). The
 * secret authorizes `channels logs reply|list|open|subscribe`.
 * {@link ChannelListener.observe} opens the same stream with an
 * existing channel secret (`S_pub` or `S_owner`) as a pure observer —
 * silent until the channel closes, then one `closed` frame; observer
 * closes close nothing.
 *
 * Auth rides the `X-OBJECTIVEAI-SIGNATURE` request header; the channel
 * secret rides `X-OBJECTIVEAI-CHANNEL-SECRET`. One listener = one
 * connection: when the socket closes the view freezes; reconnect is
 * the caller's loop. Fetch mode only for now (a viewer proxy is a
 * later migration). Unparseable events are skipped (forward compat).
 */

import { connectSse } from "./sse";

import type {
  CliChannelListenerChannelEvent,
  CliChannelListenerChannelOffer,
} from "./channel_listener";

type ChannelEvent = CliChannelListenerChannelEvent;
type ChannelOffer = CliChannelListenerChannelOffer;

// TODO: switch to the generated ChannelStreamEvent zod mirror once the
// json-schema regen lands.
type ChannelStreamEvent = { type: "secret"; secret: string } | { type: "closed" };

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
 * A live per-channel stream (`GET /channels/{id}`). An accept-opened
 * stream is the channel's LIVENESS ANCHOR — {@link close} (or losing
 * the process) closes the channel; an observer stream closes nothing.
 */
export class ChannelStream {
  /** `S_owner` for an accept-opened stream; `null` for observers. */
  readonly secret: string | null;
  #abort: AbortController;
  #closed = false;
  #closedWaiters = new Set<() => void>();

  private constructor(secret: string | null, abort: AbortController) {
    this.secret = secret;
    this.#abort = abort;
  }

  /** Whether the channel has closed (or the transport ended). */
  get isClosed(): boolean {
    return this.#closed;
  }

  /** Resolves when the channel closes (the `closed` frame) or the
   * transport ends, whichever comes first. */
  closed(): Promise<void> {
    if (this.#closed) return Promise.resolve();
    return new Promise<void>((resolve) => {
      this.#closedWaiters.add(resolve);
    });
  }

  /** Disconnect. For an accept-opened stream this CLOSES the channel
   * (terminal); for an observer it just stops watching. */
  close(): void {
    this.#abort.abort();
    this.#markClosed();
  }

  #markClosed(): void {
    if (this.#closed) return;
    this.#closed = true;
    const waiters = [...this.#closedWaiters];
    this.#closedWaiters.clear();
    for (const wake of waiters) wake();
  }

  /** Open `GET {base}/channels/{id}`: accept mode when `channelSecret`
   * is null/undefined (awaits the first `secret` frame), observer mode
   * otherwise. Rejects with the transport error — an `SseHttpError`
   * carries the HTTP status (404 unknown/withdrawn, 409 already
   * accepted, 401 unauthorized). */
  static async open(
    baseUrl: string,
    signature: string | null | undefined,
    channelId: string,
    channelSecret?: string | null,
  ): Promise<ChannelStream> {
    const abort = new AbortController();
    const events = await connectSse(
      `${baseUrl.replace(/\/+$/, "")}/channels/${encodeURIComponent(channelId)}`,
      signature,
      abort.signal,
      channelSecret ? { channelSecret } : undefined,
    );
    const accepting = !channelSecret;
    let secret: string | null = null;
    if (accepting) {
      // The FIRST frame must deliver the owner secret.
      for await (const data of events) {
        let frame: unknown;
        try {
          frame = JSON.parse(data);
        } catch {
          continue;
        }
        const event = frame as ChannelStreamEvent;
        if (event.type === "secret") {
          secret = event.secret;
          break;
        }
        abort.abort();
        throw new Error(
          `channel stream: unexpected first frame ${String(event.type)}`,
        );
      }
      if (secret === null) {
        abort.abort();
        throw new Error("channel stream ended before the owner secret");
      }
    }
    const stream = new ChannelStream(secret, abort);
    void stream.#pump(events);
    return stream;
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
        if ((frame as ChannelStreamEvent).type === "closed") {
          break;
        }
      }
    } catch {
      // Aborted or transport error — the stream is over either way.
    } finally {
      this.#markClosed();
    }
  }
}

/**
 * The materialized `/channels` offer view + per-channel stream opener —
 * construct via {@link ChannelListener.connect}.
 *
 * ```ts
 * const surface = await ChannelListener.connect(
 *   "http://127.0.0.1:49152",
 *   { signature },
 * );
 * // ...a publisher runs `channels publish --key demo --details '{}'`...
 * await surface.subscribe();               // wakes on the offer
 * const [offer] = surface.pending();
 * const stream = await surface.accept(offer.channel_id);
 * stream.secret;                           // S_owner
 * // ...use it with `channels logs reply|list|open|subscribe`;
 * // keep `stream` open — closing it closes the channel.
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
   * `http://127.0.0.1:49152`) — `/channels` and `/channels/{id}` are
   * appended. The returned listener immediately begins folding events
   * (the open-offer replay, then the `live` marker).
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
   * {@link subscribe} resolves. Open {@link ChannelStream}s are
   * independent and survive. */
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
   * Accept an open offer by OPENING its per-channel stream — the open
   * IS the accept (first-wins). Resolves once the first frame delivers
   * `S_owner` ({@link ChannelStream.secret}). KEEP THE STREAM OPEN —
   * closing it closes the channel. Rejects with an `SseHttpError`
   * carrying the status on refusal (404/409/401).
   */
  accept(channelId: string): Promise<ChannelStream> {
    return ChannelStream.open(this.#baseUrl, this.#signature, channelId);
  }

  /**
   * Observe an existing channel with a channel secret (`S_pub` or
   * `S_owner`). The stream is silent until the channel closes; closing
   * it closes nothing.
   */
  observe(channelId: string, secret: string): Promise<ChannelStream> {
    return ChannelStream.open(this.#baseUrl, this.#signature, channelId, secret);
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
