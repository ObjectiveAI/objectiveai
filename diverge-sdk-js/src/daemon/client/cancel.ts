// A scope's cancel.

import { SendError } from "../../wire/client/error.js";
import type { Handle } from "../../wire/client/handle.js";

/**
 * The one byte every cancellable scope's cancel is: the tag of the
 * scope's one channel request, which carries nothing.
 */
const CANCEL = Uint8Array.of(0);

/**
 * A scope's cancel, held beside its stream or its pending answer: a
 * logs watch, a list kept open, a message not yet taken. One channel
 * request, carrying nothing, on the scope; the daemon answers
 * nothing on the channel and finishes the scope, and what was sent
 * before the finish stands. A scope already finished is nothing to
 * cancel, and is not an error.
 */
export class Cancel {
  readonly #handle: Handle;
  readonly #scope: number;

  constructor(handle: Handle, scope: number) {
    this.#handle = handle;
    this.#scope = scope;
  }

  /** Ask the daemon to finish the scope. */
  async cancel(): Promise<void> {
    try {
      await this.#handle.sendChannelRequest(this.#scope, CANCEL);
    } catch (error) {
      if (error instanceof SendError && error.kind === "scope") {
        return;
      }
      throw error;
    }
  }
}
