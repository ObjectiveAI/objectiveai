// The `Handle` that asks the app's Rust to send.

import type { Channel } from "../client/channel.js";
import { SendError } from "../client/error.js";
import type { Handle } from "../client/handle.js";
import { Receiver } from "../client/receiver.js";
import type { Scope } from "../client/scope.js";
import { WIRE_CHANNEL_REQUEST, WIRE_CHANNEL_RESPONSE, WIRE_CHANNEL_RESPONSE_FINISH, WIRE_REQUEST, bytesOf, refusalKind } from "./commands.js";
import type { Transport } from "./transport.js";

/**
 * A Tauri channel feeding `receiver`: every message a whole frame,
 * pushed as bytes, until the empty message, the end, which closes
 * the receiver. The receiver closes on the end and on nothing else:
 * a finish frame is pushed like any other frame, and the end follows
 * it, since the Rust side's receiver closes after the finish is
 * forwarded. `onmessage` is attached here, before the command that
 * opens the channel is invoked, since a channel message and the
 * invoke's settlement are unordered across the boundary.
 */
function feeding(transport: Transport, receiver: Receiver<Uint8Array>): unknown {
  const channel = transport.channel<unknown>();
  channel.onmessage = (message) => {
    const bytes = bytesOf(message);
    if (bytes.length === 0) {
      receiver.close();
    } else {
      receiver.push(bytes);
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
 * channel's one are each a Tauri channel, closed by the end the
 * Rust side sends when its own receiver closes. So the connection
 * ending reaches a reader here exactly as it does over a socket:
 * every open receiver closes, and every later send is refused
 * `connection`. A command's refusal is thrown as the `SendError` of
 * the kind it names.
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
      throw new SendError(refusalKind(cause), cause);
    }
  }

  async sendRequest(payload: Uint8Array): Promise<Scope> {
    const responseReceiver = new Receiver<Uint8Array>();
    const requestReceiver = new Receiver<Uint8Array>();
    const responses = feeding(this.#transport, responseReceiver);
    const requests = feeding(this.#transport, requestReceiver);
    const scope = numberOf(await this.#invoke(WIRE_REQUEST, { payload: Array.from(payload), responses, requests }));
    return { scope, responseReceiver, requestReceiver };
  }

  async sendChannelRequest(scope: number, payload: Uint8Array): Promise<Channel> {
    const responseReceiver = new Receiver<Uint8Array>();
    const responses = feeding(this.#transport, responseReceiver);
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
