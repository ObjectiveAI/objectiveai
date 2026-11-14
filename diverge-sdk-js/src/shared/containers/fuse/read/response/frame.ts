// The answer to a read.

import { z } from "zod";
import { concat, text } from "../../bytes.js";
import { FuseError } from "../../error.js";

/**
 * A piece of a file: its bytes, that no file is there, or why it
 * could not be read. One byte says which — `0`, `1`, `2` — then the
 * bytes read, nothing, or the message as UTF-8. Fewer bytes than were
 * asked for is the end of the file.
 */
export const Frame = z.union([
  /** The bytes read, from the offset asked: as many as the file has there, up to the length asked. Byte `0`. */
  z.strictObject({ present: z.custom<Uint8Array>((value) => value instanceof Uint8Array) }),
  /** No file is at the path. Byte `1`. */
  z.literal("missing"),
  /** It could not be read: why. Byte `2`. */
  z.strictObject({ error: z.string() }),
]);
export type Frame = z.infer<typeof Frame>;

/** The frame as its bytes. */
export function encode(frame: Frame): Uint8Array {
  if (frame === "missing") {
    return Uint8Array.of(1);
  }
  if ("present" in frame) {
    return concat(Uint8Array.of(0), frame.present);
  }
  return concat(Uint8Array.of(2), new TextEncoder().encode(frame.error));
}

/** A frame out of its bytes. The bytes read are a view of `bytes`, not a copy. */
export function decode(bytes: Uint8Array): Frame {
  if (bytes.length === 0) {
    throw new FuseError("empty");
  }
  switch (bytes[0]) {
    case 0:
      return { present: bytes.subarray(1) };
    case 1:
      return "missing";
    case 2:
      return { error: text(bytes.subarray(1)) };
    default:
      throw new FuseError("unknown_kind", bytes[0]);
  }
}
