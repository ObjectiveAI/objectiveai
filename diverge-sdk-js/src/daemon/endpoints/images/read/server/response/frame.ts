// An images read's first answer.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../wire/decode.js";
import { Error } from "../../../../../../shared/error.js";

/**
 * Whether the daemon can supply the image: held, not found,
 * forbidden, or a failure. Exactly one, first. After `held` the scope
 * stays open and nothing more comes on it until the finish, which
 * follows the caller's cancel, the caller gone, or the source's scope
 * ending; after any other, the finish follows at once and no channel
 * is answered.
 */
export const Frame = z.union([
  /** A source the daemon asked can supply the image, and the daemon serves the scope through it. Tag `0`. */
  z.literal("held"),
  /** No source the daemon asked can. Tag `1`. */
  z.literal("not_found"),
  /** The account the request is served for holds neither `read` over providers nor `relay` over daemons; nothing was asked. Tag `2`. */
  z.literal("forbidden"),
  /** A failure. See `shared.error.Error` for why it says so little. Tag `3`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

const HELD = 0;
const NOT_FOUND = 1;
const FORBIDDEN = 2;
const ERROR = 3;

/**
 * A frame out of its payload: the tag byte, then — for the error
 * alone — the error's value. A tag that names no variant is the
 * `unexpected_tag` error.
 */
export function decode(bytes: Uint8Array): Frame {
  const [tag, rest] = splitTag(bytes);
  switch (tag) {
    case HELD:
      return "held";
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
