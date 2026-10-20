// An edit's answer.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../../wire/decode.js";
import { Error } from "../../../../../../../shared/error.js";

/**
 * An edit's answer: the daemon is as the result states, no daemon has
 * the name, a link names no provider of the caller's, forbidden, or a
 * failure.
 */
export const Frame = z.union([
  /** The daemon's links are as the request states. Tag `0`. */
  z.literal("edited"),
  /** No daemon of the caller's has the name; nothing changed. Tag `1`. */
  z.literal("not_found"),
  /** A link names a provider the caller has no record of; nothing changed. Tag `2`. */
  z.literal("no_provider"),
  /** The account the request is served for holds no grant allowing it; nothing changed. Tag `3`. */
  z.literal("forbidden"),
  /** A failure. See `shared.error.Error` for why it says so little. Tag `4`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

const EDITED = 0;
const NOT_FOUND = 1;
const NO_PROVIDER = 2;
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
    case EDITED:
      return "edited";
    case NOT_FOUND:
      return "not_found";
    case NO_PROVIDER:
      return "no_provider";
    case FORBIDDEN:
      return "forbidden";
    case ERROR:
      return { error: Error.parse(jsonOf(rest)) };
    default:
      throw new DecodeError("unexpected_tag", tag);
  }
}
