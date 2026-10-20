// An accounts add's answer.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../../../wire/decode.js";
import { Error } from "../../../../../../../../shared/error.js";

/**
 * An accounts add's answer: the account is added, no daemon has the
 * name, the daemon has the account already, forbidden, or a failure.
 */
export const Frame = z.union([
  /** The account is added to the daemon, under its name. Tag `0`. */
  z.literal("added"),
  /** No daemon of the caller's has the name; nothing changed. Tag `1`. */
  z.literal("no_daemon"),
  /** The daemon has an account under that name already; nothing changed. Tag `2`. */
  z.literal("exists"),
  /** The account the request is served for holds no grant allowing it; nothing changed. Tag `3`. */
  z.literal("forbidden"),
  /** A failure. See `shared.error.Error` for why it says so little. Tag `4`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

const ADDED = 0;
const NO_DAEMON = 1;
const EXISTS = 2;
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
    case ADDED:
      return "added";
    case NO_DAEMON:
      return "no_daemon";
    case EXISTS:
      return "exists";
    case FORBIDDEN:
      return "forbidden";
    case ERROR:
      return { error: Error.parse(jsonOf(rest)) };
    default:
      throw new DecodeError("unexpected_tag", tag);
  }
}
