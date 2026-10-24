// The `Handle` that asks the app's Rust to send.

import { CHANNEL_RESPONSE_FINISH, RESPONSE_FINISH } from "../frame/kind.js";
import { HEADER_LEN, splitHeader } from "../frame/header.js";
import type { Channel } from "../client/channel.js";
import { SendError } from "../client/error.js";
import type { Handle } from "../client/handle.js";
import { Receiver } from "../client/receiver.js";
import type { Scope } from "../client/scope.js";
import { WIRE_CHANNEL_REQUEST, WIRE_CHANNEL_RESPONSE, WIRE_CHANNEL_RESPONSE_FINISH, WIRE_REQUEST, bytesOf } from "./commands.js";
import type { Transport } from "./transport.js";

/**
 * A Tauri channel feeding `receiver`: every message a whole frame,
 * pushed as bytes; after the frame whose type is `ends` — a scope's
 * finish, or a channel's — the receiver is closed and `then` is
 * called, for whatever else that finish ends. `onmessage` is
 * attached here, before the command that opens the channel is
 * invoked, since a channel message and the invoke's settlement are
 * unordered across the boundary.
 */
function feeding(transport: Transport, receiver: Receiver<Uint8Array>, ends: number | null, then?: () => void): unknown {
  const channel = transport.channel<unknown>();
  channel.onmessage = (message) => {
    const bytes = bytesOf(message);
    receiver.push(bytes);
    if (ends !== null && bytes.length >= HEADER_LEN && splitHeader(bytes).type === ends) {
      receiver.close();
      then?.();
    }
  };
  return channel;
}

/** A number as the app's Rust answered it. */
function numberOf(answer: unknown): number {
  if (typeof answer !== "number" || !Number.isInteger(answer) || answer < 0) {
    throw new TypeError("the proxy answered with no number");
  }
  return answer;
}

/**
 * The `Handle` whose connection lives on the other side of a Tauri
 * boundary. Each send is one invoke; a scope's two receivers and a
 * channel's one are each a Tauri channel. A scope's requests
 * receiver closes with the scope's finish, since a server opens no
 * channel in a scope that is over.
 */
export class ProxyHandle implements Handle {
  readonly #transport: Transport;

  constructor(transport: Transport) {
    this.#transport = transport;
  }

  async #invoke(command: string, args: Record<string, unknown>): Promise<unknown> {
    try {
      return await this.#transport.invoke(command, args);
    } catch (cause) {
      throw new SendError("connection", cause);
    }
  }

  async sendRequest(payload: Uint8Array): Promise<Scope> {
    const responseReceiver = new Receiver<Uint8Array>();
    const requestReceiver = new Receiver<Uint8Array>();
    const responses = feeding(this.#transport, responseReceiver, RESPONSE_FINISH, () => requestReceiver.close());
    const requests = feeding(this.#transport, requestReceiver, null);
    const scope = numberOf(await this.#invoke(WIRE_REQUEST, { payload: Array.from(payload), responses, requests }));
    return { scope, responseReceiver, requestReceiver };
  }

  async sendChannelRequest(scope: number, payload: Uint8Array): Promise<Channel> {
    const responseReceiver = new Receiver<Uint8Array>();
    const responses = feeding(this.#transport, responseReceiver, CHANNEL_RESPONSE_FINISH);
    const channel = numberOf(await this.#invoke(WIRE_CHANNEL_REQUEST, { scope, payload: Array.from(payload), responses }));
    return { channel, responseReceiver };
  }

  async sendChannelResponse(scope: number, channel: number, payload: Uint8Array): Promise<void> {
    await this.#invoke(WIRE_CHANNEL_RESPONSE, { scope, channel, payload: Array.from(payload) });
  }

  async sendChannelResponseFinish(scope: number, channel: number): Promise<void> {
    await this.#invoke(WIRE_CHANNEL_RESPONSE_FINISH, { scope, channel });
  }
}

/** A handle over `transport`: what the app calls once and keeps. */
export function handle(transport: Transport): ProxyHandle {
  return new ProxyHandle(transport);
}
