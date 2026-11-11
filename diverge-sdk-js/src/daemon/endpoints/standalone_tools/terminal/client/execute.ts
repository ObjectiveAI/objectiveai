// Performing the exchange: the terminal's output, and the way in.

import { SendError } from "../../../../../wire/client/error.js";
import type { Handle } from "../../../../../wire/client/handle.js";
import { ExecuteStream } from "../../../../../daemon/client/stream.js";
import { open } from "../../../../../daemon/client/one_shot.js";
import type { Open } from "../../../../../shared/containers/terminal/open/open.js";
import * as request from "./request/index.js";
import * as channel_request from "./channel_request/index.js";
import * as response from "../server/response/index.js";

/**
 * Send the request and hand back the terminal's output — each item a
 * `response.Item`, stdout with its bytes — and the `Terminal` that
 * writes its stdin, resizes it and closes it. Leaving the stream early
 * abandons the scope, which kills the shell too.
 */
export async function execute(handle: Handle, frame: request.Frame): Promise<[ExecuteStream<response.Item>, Terminal]> {
  const scope = await open(handle, frame, request.encode);
  return [new ExecuteStream(scope, response.decode), new Terminal(handle, scope.scope)];
}

/**
 * The way into an open terminal: each call one channel request on the
 * scope, resolving once the daemon has finished it. A scope already
 * finished — the shell ended — takes nothing more, and is not an
 * error.
 */
export class Terminal {
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

  /** Write `bytes` to the shell's stdin; resolves once written. Send the next once this resolves. */
  stdin(bytes: Uint8Array): Promise<void> {
    return this.#send({ stdin: bytes }, true);
  }

  /** Set the terminal's size; resolves once set. */
  resize(size: Open): Promise<void> {
    return this.#send({ resize: size }, true);
  }

  /** Kill the shell; the scope's finish, its answer, arrives on the output. */
  close(): Promise<void> {
    return this.#send("close", false);
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
