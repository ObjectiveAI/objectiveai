// A run of bytes behind its length.

import { concat } from "./bytes.js";
import { FuseError } from "./error.js";

/**
 * `bytes` behind their length as a `u16`, big-endian: how a path or a
 * name is written where something follows it. More than 65535 bytes
 * is `length`.
 */
export function put(bytes: Uint8Array): Uint8Array {
  if (bytes.length > 0xffff) {
    throw new FuseError("length", bytes.length);
  }
  return concat(Uint8Array.of(bytes.length >> 8, bytes.length & 0xff), bytes);
}

/** The run behind its length, and what follows it; a run shorter than its length is `truncated`. */
export function take(bytes: Uint8Array): [Uint8Array, Uint8Array] {
  if (bytes.length < 2) {
    throw new FuseError("truncated");
  }
  const length = ((bytes[0] as number) << 8) | (bytes[1] as number);
  if (bytes.length < 2 + length) {
    throw new FuseError("truncated");
  }
  return [bytes.subarray(2, 2 + length), bytes.subarray(2 + length)];
}
