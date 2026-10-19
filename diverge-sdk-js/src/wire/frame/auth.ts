// The credential frame's payload.

import { utf8 } from "../encode.js";

/** The mode byte of a credential presented as itself. */
const UNBROKERED = 0;

/**
 * The payload of the one auth frame a client sends: a mode byte,
 * `0` for unbrokered, then the credential's UTF-8, running to the
 * end of the payload so that it needs no length. It is answered by
 * nothing: an accepted credential is followed by the connection
 * simply working, a rejected one by a close — wrong, expired and
 * unknown are indistinguishable from outside, which is the point.
 */
export function unbrokered(credential: string): Uint8Array {
  const text = utf8(credential);
  const out = new Uint8Array(1 + text.length);
  out[0] = UNBROKERED;
  out.set(text, 1);
  return out;
}
