// A message's answer.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../wire/decode.js";
import { Error } from "../../../../../../shared/error.js";

/**
 * A message's answer: the loop took it, the client cancelled it
 * first, forbidden, or a failure. Exactly one per scope, when the
 * fate is known; the answer, not the cancel, says which came first.
 */
export const Frame = z.union([
  /** The loop took the message. Tag `0`. */
  z.literal("delivered"),
  /** The client's cancel came first; the loop never saw the message. Tag `1`. */
  z.literal("cancelled"),
  /** The account the request is served for holds no grant allowing it; nothing changed. Tag `2`. */
  z.literal("forbidden"),
  /** A failure. See `shared.error.Error` for why it says so little. Tag `3`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

const DELIVERED = 0;
const CANCELLED = 1;
const FORBIDDEN = 2;
const ERROR = 3;

/**
 * A frame out of its payload: the tag byte, then the variant's own
 * JSON, parsed as what the tag promised — nothing for a bare
 * answer, the error's value for a failure. A tag that names no
 * variant is the `unexpected_tag` error.
 */
export function decode(bytes: Uint8Array): Frame {
  const [tag, rest] = splitTag(bytes);
  switch (tag) {
    case DELIVERED:
      return "delivered";
    case CANCELLED:
      return "cancelled";
    case FORBIDDEN:
      return "forbidden";
    case ERROR:
      return { error: Error.parse(jsonOf(rest)) };
    default:
      throw new DecodeError("unexpected_tag", tag);
  }
}
