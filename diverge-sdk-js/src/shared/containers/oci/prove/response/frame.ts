// The 32 bytes of the proof.

import { DecodeError } from "../../../../../wire/decode.js";

/**
 * The proof, verbatim: exactly 32 bytes, no header, no tag —
 * `SHA-256(nonce ‖ D)`. At most one per channel, then the finish.
 */
export type Frame = Uint8Array;

/** The proof's length. */
const LENGTH = 32;

/** The 32 bytes; a payload of any other length is the `body` error. */
export function decode(bytes: Uint8Array): Frame {
  if (bytes.length !== LENGTH) {
    throw new DecodeError("body");
  }
  return bytes;
}
