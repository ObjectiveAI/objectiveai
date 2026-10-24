// One answer that may be waited for, or cancelled.

import type { Decode, Encode } from "../../wire/index.js";
import type { Handle, Scope } from "../../wire/client/index.js";
import { Cancel } from "./cancel.js";
import { answer, open } from "./one_shot.js";

/**
 * An answer not yet come: a message sent and not yet taken, say.
 * `wait` reads it; `cancel` asks the daemon to finish the scope
 * instead, and the answer then says which came first.
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

  /** The one answer. */
  wait(): Promise<A> {
    return answer(this.#scope, this.#decode);
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
