// A providers add's answer.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../wire/decode.js";
import { Error } from "../../../../../../shared/error.js";

/** Exactly one per scope, before the finish. */
export const Frame = z.union([
  /** The record is kept. Tag `0`. */
  z.literal("added"),
  /** A record of that identity is kept already; nothing changed. Tag `1`. */
  z.literal("exists"),
  /** The account the request is served for holds no grant allowing it; nothing changed. Tag `2`. */
  z.literal("forbidden"),
  /** A failure; nothing changed. See `shared.error.Error` for why it says so little. Tag `3`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

/** A frame out of its payload: the tag byte, and the error's JSON for a failure. */
export function decode(bytes: Uint8Array): Frame {
  const [tag, rest] = splitTag(bytes);
  switch (tag) {
    case 0:
      return "added";
    case 1:
      return "exists";
    case 2:
      return "forbidden";
    case 3:
      return { error: Error.parse(jsonOf(rest)) };
    default:
      throw new DecodeError("unexpected_tag", tag);
  }
}
