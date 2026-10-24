// An add's answer.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../../wire/decode.js";
import { Error } from "../../../../../../../shared/error.js";

/**
 * An add's answer: the daemon is added, the name is a daemon's
 * already, a link names no provider of the caller's, forbidden, or a
 * failure.
 */
export const Frame = z.union([
  /** The daemon is added, under its name. Tag `0`. */
  z.literal("added"),
  /** The name is a daemon's of the caller's already; nothing changed. Tag `1`. */
  z.literal("exists"),
  /** A link names a provider the caller has no record of; nothing changed. Tag `2`. */
  z.literal("no_provider"),
  /** The account the request is served for holds no grant allowing it; nothing changed. Tag `3`. */
  z.literal("forbidden"),
  /** A failure. See `shared.error.Error` for why it says so little. Tag `4`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

const ADDED = 0;
const EXISTS = 1;
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
    case ADDED:
      return "added";
    case EXISTS:
      return "exists";
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
