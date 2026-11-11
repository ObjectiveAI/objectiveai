// Performing the exchange: the answer, and the way in.

import { SendError } from "../../../../../wire/client/error.js";
import type { Handle } from "../../../../../wire/client/handle.js";
import { ExecuteStream } from "../../../../../daemon/client/stream.js";
import { open } from "../../../../../daemon/client/one_shot.js";
import * as request from "./request/index.js";
import * as channel_request from "./channel_request/index.js";
import * as response from "../server/response/index.js";

/**
 * Send the request and hand back its answer — each item a
 * `response.Item`, a body with its bytes — and the `Frontend` that
 * writes its body and cancels it. Leaving the stream early abandons the
 * scope, which drops the request too.
 */
export async function execute(handle: Handle, frame: request.Frame): Promise<[ExecuteStream<response.Item>, Frontend]> {
  const scope = await open(handle, frame, request.encode);
  return [new ExecuteStream(scope, response.decode), new Frontend(handle, scope.scope)];
}

/**
 * The way into an open frontend request: each call one channel request
 * on the scope. A scope already finished takes nothing more, and is not
 * an error.
 */
export class Frontend {
  readonly #handle: Handle;
  readonly #scope: number;

  constructor(handle: Handle, scope: number) {
    this.#handle = handle;
    this.#scope = scope;
  }

  /** The scope's number, for a caller that keeps its own books. */
  get scope(): number {
    return this.#scope;
  }

  /** Send a piece of the body; resolves once the daemon has taken it. Send the next once this resolves. */
  body(bytes: Uint8Array): Promise<void> {
    return this.#send({ body: bytes }, true);
  }

  /** End the body; resolves once the daemon has taken it. */
  end(): Promise<void> {
    return this.#send("end", true);
  }

  /** Drop the request; the scope's finish, its answer, arrives on the answer. */
  cancel(): Promise<void> {
    return this.#send("cancel", false);
  }

  async #send(frame: channel_request.Frame, wait: boolean): Promise<void> {
    try {
      const channel = await this.#handle.sendChannelRequest(this.#scope, channel_request.encode(frame));
      // The channel's one frame is its finish.
      if (wait) {
        await channel.responseReceiver.next();
      }
      channel.responseReceiver.abandon();
    } catch (error) {
      if (error instanceof SendError && error.kind === "scope") {
        return;
      }
      throw error;
    }
  }
}
