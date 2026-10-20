// An insert's answer.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../wire/decode.js";
import { Error } from "../../../../../../shared/error.js";

/**
 * An insert's answer: the key holds the value, no such key or slot,
 * the slot is locked, forbidden, or a failure.
 */
export const Frame = z.union([
  /** The key holds the value: made, appended to, or a slot replaced. Tag `0`. */
  z.literal("inserted"),
  /** The slot named is not there — no key has the name, or the key has no slot of the number; nothing changed. Tag `1`. */
  z.literal("not_found"),
  /** A running container holds the slot named as a `rotating` secret, and it was left as it is; nothing changed. Tag `2`. */
  z.literal("locked"),
  /** The account the request is served for holds no grant allowing it; nothing changed. Tag `3`. */
  z.literal("forbidden"),
  /** A failure. See `shared.error.Error` for why it says so little. Tag `4`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

const INSERTED = 0;
const NOT_FOUND = 1;
const LOCKED = 2;
const FORBIDDEN = 3;
const ERROR = 4;

/**
 * A frame out of its payload: the tag byte, then the variant's own
 * JSON, parsed as what the tag promised — nothing for a bare
 * answer, the error's value for a failure. A tag that names no
 * variant is the `unexpected_tag` error.
 */
export function decode(bytes: Uint8Array): Frame {
  const [tag, rest] = splitTag(bytes);
  switch (tag) {
    case INSERTED:
      return "inserted";
    case NOT_FOUND:
      return "not_found";
    case LOCKED:
      return "locked";
    case FORBIDDEN:
      return "forbidden";
    case ERROR:
      return { error: Error.parse(jsonOf(rest)) };
    default:
      throw new DecodeError("unexpected_tag", tag);
  }
}
