/**
 * Materialized consumer of the cli daemon's `/channels` endpoint — the
 * OFFER lifecycle side of duplex channels: the JS mirror of the Rust
 * SDK's `daemon::channel_listener::ChannelListener`.
 *
 * `GET /channels` (the `daemon_channels` proxy command in viewer
 * mode) is the offer lifecycle stream and nothing else: the listener
 * folds `offer` (insert into the pending map), `offer_withdrawn`
 * (remove), and the `live` caught-up marker. No secrets ride this
 * stream.
 *
 * Accepting is {@link Client.acceptChannel} — the client-level
 * `POST /channels/{id}/accept` (first-wins) whose `200` body carries
 * the owner secret (`S_owner`). The daemon tracks no liveness; a
 * channel stays open until someone runs `channels close` with either
 * of its secrets.
 *
 * One listener = one connection: when the socket closes the view
 * freezes; reconnect is the caller's loop — mint a new listener from
 * the client. Unparseable events are skipped (forward compat).
 */

import type {
  DaemonChannelListenerChannelEvent,
  DaemonChannelListenerChannelOffer,
} from "./channel_listener";
import { openStream, type ClientMode } from "./mode";

type ChannelEvent = DaemonChannelListenerChannelEvent;
type ChannelOffer = DaemonChannelListenerChannelOffer;

/**
 * The materialized `/channels` offer view — minted by
 * {@link Client.channelListener}; resolves only once the stream has
 * OPENED.
 *
 * ```ts
 * const client = new Client("http://127.0.0.1:49152", { signature });
 * const surface = await client.channelListener();
 * // ...a publisher runs `channels publish --key demo --details '{}'`...
 * await surface.subscribe();               // wakes on the offer
 * const [offer] = surface.pending();
 * const sOwner = await client.acceptChannel(offer.channel_id);
 * ```
 */
export class ChannelListener {
  #abort: AbortController;
  #closed = false;
  /** `channel_id → offer` — the currently OPEN offers. */
  #offers = new Map<string, ChannelOffer>();
  /** Resolvers for the in-flight {@link subscribe} promises. */
  #waiters = new Set<() => void>();

  private constructor(abort: AbortController) {
    this.#abort = abort;
  }

  /** @internal — minted by `Client.channelListener`. */
  static async _connect(mode: ClientMode): Promise<ChannelListener> {
    const abort = new AbortController();
    const events = await openStream(mode, abort.signal, {
      route: "/channels",
      viewerCommand: "daemon_channels",
    });
    const listener = new ChannelListener(abort);
    void listener.#pump(events);
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

  #onFrame(frame: unknown): void {
    if (typeof frame !== "object" || frame === null) return;
    if (typeof (frame as { type?: unknown }).type !== "string") return;
    if (this.#apply(frame as ChannelEvent)) {
      this.#notify();
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
