// The read loop: frames off the socket, forwarded to whoever waits.

import { AUTH, CHANNEL_REQUEST, CHANNEL_RESPONSE, CHANNEL_RESPONSE_FINISH, RESPONSE, RESPONSE_FINISH } from "../frame/kind.js";
import { HEADER_LEN, splitHeader } from "../frame/header.js";
import type { Connection } from "../connection.js";
import type { Registry } from "./registry.js";

/**
 * Reads every frame the socket yields and forwards it WHOLE —
 * header included, since the far side needs the type to tell a
 * finish from a response — to the receiver registered for it. A
 * response or finish goes to the scope's answer; a channel request
 * to the scope's requests; a channel response or finish to the
 * channel this end opened. A finish also closes what it ends: the
 * scope and everything under it, or the one channel.
 *
 * A frame nobody is waiting for is dropped, silently: a header too
 * short to read, a type this side does not define, a scope with no
 * entry, a channel with no entry inside a scope that has one. A
 * second credential after the handshake ends routing, as it ends
 * the connection. The socket ending closes every scope.
 */
export class Router {
  readonly #connection: Connection;
  readonly #registry: Registry;

  constructor(connection: Connection, registry: Registry) {
    this.#connection = connection;
    this.#registry = registry;
  }

  /** Run until the socket ends, then close every scope. */
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
        switch (type) {
          case RESPONSE:
            entry.responseReceiver.push(bytes);
            break;
          case RESPONSE_FINISH:
            entry.responseReceiver.push(bytes);
            this.#registry.closeScope(scope);
            break;
          case CHANNEL_REQUEST:
            entry.requestReceiver.push(bytes);
            break;
          case CHANNEL_RESPONSE:
            entry.channels.get(channel)?.push(bytes);
            break;
          case CHANNEL_RESPONSE_FINISH:
            entry.channels.get(channel)?.push(bytes);
            this.#registry.closeChannel(scope, channel);
            break;
          default:
            break;
        }
      }
    } finally {
      this.#registry.closeAll();
    }
  }
}
