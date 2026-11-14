// The answer to a stat.

import { z } from "zod";
import { concat, text } from "../../bytes.js";
import { FuseError } from "../../error.js";
import * as stat from "../stat.js";

/**
 * What is at the path: its stat, that nothing is there, or why it
 * could not be looked at. One byte says which — `0`, `1`, `2` — then
 * the stat's bytes, nothing, or the message as UTF-8.
 */
export const Frame = z.union([
  /** Something is there: its stat. Byte `0`. */
  z.strictObject({ present: stat.Stat }),
  /** Nothing is at the path. Byte `1`. */
  z.literal("missing"),
  /** It could not be looked at: why. Byte `2`. */
  z.strictObject({ error: z.string() }),
]);
export type Frame = z.infer<typeof Frame>;

/** The frame as its bytes. */
export function encode(frame: Frame): Uint8Array {
  if (frame === "missing") {
    return Uint8Array.of(1);
  }
  if ("present" in frame) {
    return concat(Uint8Array.of(0), stat.encode(frame.present));
  }
  return concat(Uint8Array.of(2), new TextEncoder().encode(frame.error));
}

/** A frame out of its bytes. */
export function decode(bytes: Uint8Array): Frame {
  if (bytes.length === 0) {
    throw new FuseError("empty");
  }
  switch (bytes[0]) {
    case 0:
      return { present: stat.decode(bytes.subarray(1)) };
    case 1:
      return "missing";
    case 2:
      return { error: text(bytes.subarray(1)) };
    default:
      throw new FuseError("unknown_kind", bytes[0]);
  }
}
