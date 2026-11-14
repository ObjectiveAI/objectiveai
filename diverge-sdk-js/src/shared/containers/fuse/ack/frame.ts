// The answer to an ask that changes something.

import { z } from "zod";
import { concat, text } from "../bytes.js";
import { FuseError } from "../error.js";

/**
 * Whether a change was made: what a `write`, a `truncate`, a
 * `setattr`, a `remove`, a `rename` and a `mkdir` are each answered
 * with. One byte says which — `0` done, `1` not done and why, `2` the
 * volume is read-only — and the message of an error is every byte
 * after it, as UTF-8.
 */
export const Frame = z.union([
  /** The change was made. Byte `0`. */
  z.literal("ok"),
  /** The change was not made: why, in the server's words. Byte `1`. */
  z.strictObject({ error: z.string() }),
  /** The volume takes no change: it is served read-only. Byte `2`. */
  z.literal("read_only"),
]);
export type Frame = z.infer<typeof Frame>;

/** The frame as its bytes. */
export function encode(frame: Frame): Uint8Array {
  if (frame === "ok") {
    return Uint8Array.of(0);
  }
  if (frame === "read_only") {
    return Uint8Array.of(2);
  }
  return concat(Uint8Array.of(1), new TextEncoder().encode(frame.error));
}

/** A frame out of its bytes. */
export function decode(bytes: Uint8Array): Frame {
  if (bytes.length === 0) {
    throw new FuseError("empty");
  }
  switch (bytes[0]) {
    case 0:
      return "ok";
    case 1:
      return { error: text(bytes.subarray(1)) };
    case 2:
      return "read_only";
    default:
      throw new FuseError("unknown_kind", bytes[0]);
  }
}
