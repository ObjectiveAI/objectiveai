// A delete's answer.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../wire/decode.js";
import { Error } from "../../../../../../shared/error.js";

/**
 * A delete's answer: the key or the value is gone, no such key or
 * slot, a record binds the key, a container holds the value, the
 * value is the key's last, forbidden, or a failure.
 */
export const Frame = z.union([
  /** The key, or the value named, is gone. Tag `0`. */
  z.literal("deleted"),
  /** No key has the name, or the key has no slot of the number; nothing changed. Tag `1`. */
  z.literal("not_found"),
  /** An agent or a tool binds the key in its `secrets`; nothing changed. Tag `2`. */
  z.literal("in_use"),
  /** A running container holds the value named — or, for a whole key, one of its values; nothing changed. Tag `3`. */
  z.literal("locked"),
  /** The slot named is the key's only one; nothing changed. A key holds at least one value: delete the key instead. Tag `4`. */
  z.literal("last"),
  /** The account the request is served for holds no grant allowing it; nothing changed. Tag `5`. */
  z.literal("forbidden"),
  /** A failure. See `shared.error.Error` for why it says so little. Tag `6`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

const DELETED = 0;
const NOT_FOUND = 1;
const IN_USE = 2;
const LOCKED = 3;
const LAST = 4;
const FORBIDDEN = 5;
const ERROR = 6;

/**
 * A frame out of its payload: the tag byte, then the variant's own
 * JSON, parsed as what the tag promised — nothing for a bare
 * answer, the error's value for a failure. A tag that names no
 * variant is the `unexpected_tag` error.
 */
export function decode(bytes: Uint8Array): Frame {
  const [tag, rest] = splitTag(bytes);
  switch (tag) {
    case DELETED:
      return "deleted";
    case NOT_FOUND:
      return "not_found";
    case IN_USE:
      return "in_use";
    case LOCKED:
      return "locked";
    case LAST:
      return "last";
    case FORBIDDEN:
      return "forbidden";
    case ERROR:
      return { error: Error.parse(jsonOf(rest)) };
    default:
      throw new DecodeError("unexpected_tag", tag);
  }
}
