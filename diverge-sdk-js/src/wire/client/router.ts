// The read loop: frames in, and off to whoever registered for them.

import { AUTH, CHANNEL_REQUEST, CHANNEL_RESPONSE, CHANNEL_RESPONSE_FINISH, CREDIT, RESPONSE, RESPONSE_FINISH } from "../frame/kind.js";
import { HEADER_LEN, splitHeader } from "../frame/header.js";
import type { Connection } from "../connection.js";
import type { Receiver } from "./receiver.js";
import type { Registry } from "./registry.js";
import { DEPTH, REQUESTS, REQUESTS_ABANDONED, Window } from "./window.js";

/**
 * One connection's inbound half. Reads frames and forwards each one
 * WHOLE, header included, to the receiver its scope and channel name:
 * a response or its finish to the scope's own, a channel request the
 * server opens to the scope's request receiver — its window made
 * first, for whoever answers it — and a channel response or its finish
 * to the channel this end opened. A credit is applied to the window it
 * names. A frame whose scope or channel is not open is dropped, as is
 * a runt; an auth frame after the handshake ends the loop; a frame
 * that finds its receiver full broke the credit rules and ends the
 * connection. When the loop ends every receiver closes and every
 * window ends.
 */
export class Router {
  readonly #connection: Connection;
  readonly #registry: Registry;

  constructor(connection: Connection, registry: Registry) {
    this.#connection = connection;
    this.#registry = registry;
  }

  /** Read until the connection ends. */
  async run(): Promise<void> {
    try {
      for await (const bytes of this.#connection.frames) {
        if (bytes.length < HEADER_LEN) {
          continue;
        }
        const { type, scope, channel } = splitHeader(bytes);
        if (type === AUTH) {
          return;
        }
        const entry = this.#registry.get(scope);
        if (entry === undefined) {
          continue;
        }
        let filed = true;
        switch (type) {
          case RESPONSE:
            filed = file(entry.responseReceiver, bytes);
            break;
          case RESPONSE_FINISH:
            filed = file(entry.responseReceiver, bytes);
            this.#registry.closeScope(scope);
            break;
          case CHANNEL_REQUEST:
            entry.answers.set(channel, new Window());
            filed = file(entry.requestReceiver, bytes);
            break;
          case CHANNEL_RESPONSE:
            filed = file(entry.channels.get(channel), bytes);
            break;
          case CHANNEL_RESPONSE_FINISH:
            filed = file(entry.channels.get(channel), bytes);
            this.#registry.closeChannel(scope, channel);
            break;
          case CREDIT: {
            const kind = bytes[HEADER_LEN];
            if (kind === undefined) {
              break;
            }
            const window = kind === REQUESTS || kind === REQUESTS_ABANDONED ? entry.asked : entry.answers.get(channel);
            window?.apply(kind);
            break;
          }
          default:
            break;
        }
        if (!filed) {
          this.#connection.close();
          return;
        }
      }
    } finally {
      this.#registry.closeAll();
    }
  }
}

/** File a frame (rules 5 and 6 in `window.ts`): `false` if its receiver was full. */
function file(receiver: Receiver<Uint8Array> | undefined, bytes: Uint8Array): boolean {
  if (receiver === undefined || receiver.closed) {
    return true;
  }
  if (receiver.depth >= DEPTH) {
    return false;
  }
  receiver.push(bytes);
  return true;
}
