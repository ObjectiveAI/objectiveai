// One answer that may be waited for, or cancelled.

import type { Decode, Encode } from "../../wire/index.js";
import { decodeServerFrame } from "../../wire/frame/server.js";
import type { Handle, Scope } from "../../wire/client/index.js";
import { Cancel } from "./cancel.js";
import { OneShotError } from "./error.js";
import { open } from "./one_shot.js";

/**
 * An answer not yet come: a message sent and not yet taken, say.
 * `wait` reads it, or the finish with none before it; `cancel` asks
 * the daemon to finish the scope instead, and whether an answer comes
 * then says which came first.
 */
export class Pending<A> {
  readonly #scope: Scope;
  readonly #decode: Decode<A>;
  readonly #cancel: Cancel;

  constructor(handle: Handle, scope: Scope, decode: Decode<A>) {
    this.#scope = scope;
    this.#decode = decode;
    this.#cancel = new Cancel(handle, scope.scope);
  }

  /**
   * The one answer, or `null`. The first frame on the scope decides:
   *
   * 1. A response: the answer, decoded, whichever it is; the finish
   *    after it is read by nobody.
   * 2. The finish: no answer before it — the bare finish, `null`. A
   *    cancel in time ends a scope so, and so does a request the
   *    daemon could not serve. A cancel too late changes nothing, and
   *    the answer comes as rule 1.
   * 3. Any other frame: the `misrouted` error.
   * 4. Nothing, the connection gone: the `closed` error.
   */
  async wait(): Promise<A | null> {
    const scope = this.#scope;
    // One frame decides; the rest is read by nobody, and the server is
    // told so.
    scope.requestReceiver.abandon();
    const bytes = await scope.responseReceiver.next();
    scope.responseReceiver.abandon();
    if (bytes === undefined) {
      throw new OneShotError("closed");
    }
    let frame;
    try {
      frame = decodeServerFrame(bytes);
    } catch (cause) {
      throw new OneShotError("frame", cause);
    }
    switch (frame.kind) {
      case "response":
        try {
          return this.#decode(frame.payload);
        } catch (cause) {
          throw new OneShotError("response", cause);
        }
      case "response_finish":
        return null;
      default:
        throw new OneShotError("misrouted");
    }
  }

  /** Ask the daemon to finish the scope without it. */
  cancel(): Promise<void> {
    return this.#cancel.cancel();
  }
}

/** Send `request` and hand back its pending answer. */
export async function execute<Q, A>(handle: Handle, request: Q, encode: Encode<Q>, decode: Decode<A>): Promise<Pending<A>> {
  const scope = await open(handle, request, encode);
  return new Pending(handle, scope, decode);
}
