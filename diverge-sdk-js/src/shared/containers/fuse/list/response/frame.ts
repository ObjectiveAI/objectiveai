// The answer to a list.

import { z } from "zod";
import { concat, takeU32, text, u32 } from "../../bytes.js";
import * as entry from "../../entry.js";
import { FuseError } from "../../error.js";

/**
 * A directory's entries: every one, that no directory is there, or
 * why it could not be listed. One byte says which — `0`, `1`, `2` —
 * then the count as a `u32`, big-endian, and each entry; nothing; or
 * the message as UTF-8.
 */
export const Frame = z.union([
  /** The directory's entries, whole. Byte `0`. */
  z.strictObject({ entries: z.array(entry.Entry) }),
  /** No directory is at the path. Byte `1`. */
  z.literal("missing"),
  /** It could not be listed: why. Byte `2`. */
  z.strictObject({ error: z.string() }),
]);
export type Frame = z.infer<typeof Frame>;

/** The frame as its bytes. */
export function encode(frame: Frame): Uint8Array {
  if (frame === "missing") {
    return Uint8Array.of(1);
  }
  if ("entries" in frame) {
    return concat(Uint8Array.of(0), u32(frame.entries.length), ...frame.entries.map(entry.encode));
  }
  return concat(Uint8Array.of(2), new TextEncoder().encode(frame.error));
}

/** A frame out of its bytes. */
export function decode(bytes: Uint8Array): Frame {
  if (bytes.length === 0) {
    throw new FuseError("empty");
  }
  switch (bytes[0]) {
    case 0: {
      const count = takeU32(bytes, 1);
      let rest = bytes.subarray(5);
      const entries: entry.Entry[] = [];
      for (let read = 0; read < count; read += 1) {
        const [one, after] = entry.decode(rest);
        entries.push(one);
        rest = after;
      }
      return { entries };
    }
    case 1:
      return "missing";
    case 2:
      return { error: text(bytes.subarray(1)) };
    default:
      throw new FuseError("unknown_kind", bytes[0]);
  }
}
