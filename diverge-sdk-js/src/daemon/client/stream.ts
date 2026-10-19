// One request, a stream of answers.

import type { Decode, Encode } from "../../wire/index.js";
import { decodeServerFrame } from "../../wire/frame/server.js";
import type { Handle, Scope } from "../../wire/client/index.js";
import { Cancel } from "./cancel.js";
import { StreamError } from "./error.js";
import { open } from "./one_shot.js";

/**
 * The responses on a scope, each decoded, until the finish — which
 * ends the iteration and is never an item. The connection ending
 * first is the `closed` error; there is no timeout, since a quiet
 * scope is a scope still running. A stream that is not read grows a
 * queue nobody drains: read what you asked for, or cancel it.
 */
export class ExecuteStream<T> implements AsyncIterable<T> {
  readonly #scope: Scope;
  readonly #decode: Decode<T>;

  constructor(scope: Scope, decode: Decode<T>) {
    this.#scope = scope;
    this.#decode = decode;
  }

  /** The scope's number, for a cancel. */
  get scope(): number {
    return this.#scope.scope;
  }

  /** The next item, or `undefined` at the finish. */
  async next(): Promise<T | undefined> {
    const bytes = await this.#scope.responseReceiver.next();
    if (bytes === undefined) {
      throw new StreamError("closed");
    }
    let frame;
    try {
      frame = decodeServerFrame(bytes);
    } catch (cause) {
      throw new StreamError("frame", cause);
    }
    switch (frame.kind) {
      case "response":
        try {
          return this.#decode(frame.payload);
        } catch (cause) {
          throw new StreamError("response", cause);
        }
      case "response_finish":
        return undefined;
      default:
        throw new StreamError("misrouted");
    }
  }

  async *[Symbol.asyncIterator](): AsyncIterator<T> {
    for (;;) {
      const item = await this.next();
      if (item === undefined) {
        return;
      }
      yield item;
    }
  }
}

/** Send `request` and hand back its stream of answers. */
export async function execute<Q, T>(handle: Handle, request: Q, encode: Encode<Q>, decode: Decode<T>): Promise<ExecuteStream<T>> {
  const scope = await open(handle, request, encode);
  return new ExecuteStream(scope, decode);
}

/**
 * Send `request` and hand back its stream of answers and the cancel
 * that ends it: one channel request on the scope, after which the
 * daemon finishes the scope and what was sent before stands.
 */
export async function executeCancellable<Q, T>(
  handle: Handle,
  request: Q,
  encode: Encode<Q>,
  decode: Decode<T>,
): Promise<[ExecuteStream<T>, Cancel]> {
  const scope = await open(handle, request, encode);
  return [new ExecuteStream(scope, decode), new Cancel(handle, scope.scope)];
}
