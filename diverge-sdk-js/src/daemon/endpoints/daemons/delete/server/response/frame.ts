// A daemons delete's answer.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../wire/decode.js";
import { Error } from "../../../../../../shared/error.js";

/** Exactly one per scope, before the finish. */
export const Frame = z.union([
  /** The record is gone. Tag `0`. */
  z.literal("deleted"),
  /** No record has the name; nothing changed. Tag `1`. */
  z.literal("not_found"),
  /** A connected tool names it; nothing changed. Tag `2`. */
  z.literal("in_use"),
  /** The account the request is served for holds no grant allowing it; nothing changed. Tag `3`. */
  z.literal("forbidden"),
  /** A failure; nothing changed. See `shared.error.Error` for why it says so little. Tag `4`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

/** A frame out of its payload: the tag byte, and the error's JSON for a failure. */
export function decode(bytes: Uint8Array): Frame {
  const [tag, rest] = splitTag(bytes);
  switch (tag) {
    case 0:
      return "deleted";
    case 1:
      return "not_found";
    case 2:
      return "in_use";
    case 3:
      return "forbidden";
    case 4:
      return { error: Error.parse(jsonOf(rest)) };
    default:
      throw new DecodeError("unexpected_tag", tag);
  }
}
