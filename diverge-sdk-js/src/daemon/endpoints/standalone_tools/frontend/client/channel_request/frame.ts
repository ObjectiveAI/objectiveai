// The channels a client opens on a frontend request.

import { tagged } from "../../../../../../wire/encode.js";

/**
 * What a client puts into a frontend request: `body`, tag `0`, a piece
 * of the request's body — after a `101`, of what the page sends — as it
 * is; `end`, tag `1`, the body done; `cancel`, tag `2`, the request
 * dropped, answered by the scope's finish. A `body` and an `end` are
 * each answered by the channel's finish once taken, and the next `body`
 * is sent once the last is finished, which keeps them in order.
 */
export type Frame = { readonly body: Uint8Array } | "end" | "cancel";

const BODY = 0;
const END = 1;
const CANCEL = 2;

/** One byte of tag, then the bytes or nothing. */
export function encode(frame: Frame): Uint8Array {
  if (frame === "end") {
    return Uint8Array.of(END);
  }
  if (frame === "cancel") {
    return Uint8Array.of(CANCEL);
  }
  return tagged(BODY, frame.body);
}
