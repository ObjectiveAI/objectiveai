// The write half over a real socket: minting and sending.

import type { Connection } from "../connection.js";
import { encodeClientFrame } from "../frame/client.js";
import type { Channel } from "./channel.js";
import { SendError } from "./error.js";
import type { Handle } from "./handle.js";
import type { Registry } from "./registry.js";
import type { Scope } from "./scope.js";

/** The last `u32`; a counter past it wraps to one. */
const MAX = 0xffff_ffff;

/**
 * The `Handle` that speaks the wire itself. Scopes are minted from
 * one upward, each number skipped while a scope of that number is
 * still open, and so are this end's channels inside each scope;
 * minting a number, registering it, and writing the frame that
 * spends it are one act, so nothing that answers can arrive before
 * its receiver exists. Scope and channel `0` are never minted:
 * channel `0` is the scope's own answer, and scope `0` is the auth
 * frame's.
 */
export class SocketHandle implements Handle {
  readonly #connection: Connection;
  readonly #registry: Registry;
  #scopeCounter = 0;
  readonly #channelCounters = new Map<number, number>();

  constructor(connection: Connection, registry: Registry) {
    this.#connection = connection;
    this.#registry = registry;
  }

  #mintScope(): number {
    for (;;) {
      this.#scopeCounter = this.#scopeCounter >= MAX ? 1 : this.#scopeCounter + 1;
      if (!this.#registry.has(this.#scopeCounter)) {
        return this.#scopeCounter;
      }
    }
  }

  #mintChannel(scope: number, live: ReadonlySet<number>): number {
    let counter = this.#channelCounters.get(scope) ?? 0;
    for (;;) {
      counter = counter >= MAX ? 1 : counter + 1;
      if (!live.has(counter)) {
        this.#channelCounters.set(scope, counter);
        return counter;
      }
    }
  }

  #write(bytes: Uint8Array): void {
    try {
      this.#connection.send(bytes);
    } catch (cause) {
      throw new SendError("connection", cause);
    }
  }

  async sendRequest(payload: Uint8Array): Promise<Scope> {
    const scope = this.#mintScope();
    const entry = this.#registry.open(scope);
    try {
      this.#write(encodeClientFrame({ kind: "request", scope, payload }));
    } catch (error) {
      this.#registry.closeScope(scope);
      throw error;
    }
    return { scope, responseReceiver: entry.responseReceiver, requestReceiver: entry.requestReceiver };
  }

  async sendChannelRequest(scope: number, payload: Uint8Array): Promise<Channel> {
    const entry = this.#registry.get(scope);
    if (entry === undefined) {
      throw new SendError("scope");
    }
    const channel = this.#mintChannel(scope, new Set(entry.channels.keys()));
    const responseReceiver = this.#registry.openChannel(scope, channel);
    if (responseReceiver === undefined) {
      throw new SendError("scope");
    }
    try {
      this.#write(encodeClientFrame({ kind: "channel_request", scope, channel, payload }));
    } catch (error) {
      this.#registry.closeChannel(scope, channel);
      throw error;
    }
    return { channel, responseReceiver };
  }

  async sendChannelResponse(scope: number, channel: number, payload: Uint8Array): Promise<void> {
    this.#write(encodeClientFrame({ kind: "channel_response", scope, channel, payload }));
  }

  async sendChannelResponseFinish(scope: number, channel: number): Promise<void> {
    this.#write(encodeClientFrame({ kind: "channel_response_finish", scope, channel }));
  }

  /** Close the connection; every scope on it is over. */
  close(): void {
    this.#connection.close();
  }
}
