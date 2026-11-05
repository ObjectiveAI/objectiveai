// A delete's answer.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../wire/decode.js";
import { Error } from "../../../../../../shared/error.js";

/**
 * A delete's answer: the account is gone — every connection a client
 * had as it closed — no account is the one named, forbidden, or a
 * failure. Exactly one per scope, before the finish.
 */
export const Frame = z.union([
  /** The account is gone, and every connection a client had as it closed. Tag `0`. */
  z.literal("deleted"),
  /** No account is the one named; nothing changed. Tag `1`. */
  z.literal("not_found"),
  /** The account the request is served for holds no grant allowing it; nothing changed. Tag `2`. */
  z.literal("forbidden"),
  /** A failure. See `shared.error.Error` for why it says so little. Tag `3`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

const DELETED = 0;
const NOT_FOUND = 1;
const FORBIDDEN = 2;
const ERROR = 3;

/**
 * A frame out of its payload: the tag byte, then the error's value for a
 * failure. A tag that names no variant is the `unexpected_tag` error.
 */
export function decode(bytes: Uint8Array): Frame {
  const [tag, rest] = splitTag(bytes);
  switch (tag) {
    case DELETED:
      return "deleted";
    case NOT_FOUND:
      return "not_found";
    case FORBIDDEN:
      return "forbidden";
    case ERROR:
      return { error: Error.parse(jsonOf(rest)) };
    default:
      throw new DecodeError("unexpected_tag", tag);
  }
}
