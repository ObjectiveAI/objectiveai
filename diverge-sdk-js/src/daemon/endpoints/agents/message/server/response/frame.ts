// A message's answer.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../wire/decode.js";
import { Error } from "../../../../../../shared/error.js";

/**
 * A message's answer: the loop took it, forbidden, or a failure. At
 * most one per scope, when the fate is known, before the finish. A
 * cancel in time is answered by none: the daemon finishes the scope
 * with no response before the finish — the message is gone and the
 * loop never saw it. A cancel after the loop took the message changes
 * nothing, and the answer is `delivered` all the same: whether an
 * answer precedes the finish, not the cancel, says which came first.
 */
export const Frame = z.union([
  /** The loop took the message. Tag `0`. */
  z.literal("delivered"),
  /** The account the request is served for holds no grant allowing it; nothing changed. Tag `1`. */
  z.literal("forbidden"),
  /** A failure. See `shared.error.Error` for why it says so little. Tag `2`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

const DELIVERED = 0;
const FORBIDDEN = 1;
const ERROR = 2;

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
    case FORBIDDEN:
      return "forbidden";
    case ERROR:
      return { error: Error.parse(jsonOf(rest)) };
    default:
      throw new DecodeError("unexpected_tag", tag);
  }
}
