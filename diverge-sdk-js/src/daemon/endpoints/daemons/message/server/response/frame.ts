// A message's answer.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../wire/decode.js";
import { Error } from "../../../../../../shared/error.js";

/**
 * A message's answer, at most one per scope, when the fate is known,
 * before the finish: delivered — the receiver's reader took it — not
 * listening, not found, forbidden, or a failure. A cancel in time is
 * answered by none: the daemon finishes the scope with no response
 * before the finish.
 */
export const Frame = z.union([
  /** The receiver's reader took the message. Tag `0`. */
  z.literal("delivered"),
  /** Listed through a provider the account may send through, but listening through none. Tag `1`. */
  z.literal("not_listening"),
  /** No connected provider's discover listing holds the identity. Tag `2`. */
  z.literal("not_found"),
  /** No grant allows it, or the identity is listed only through providers the grant does not reach. Tag `3`. */
  z.literal("forbidden"),
  /** A failure. See `shared.error.Error` for why it says so little. Tag `4`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

const DELIVERED = 0;
const NOT_LISTENING = 1;
const NOT_FOUND = 2;
const FORBIDDEN = 3;
const ERROR = 4;

/**
 * A frame out of its payload: the tag byte, then the error's JSON for a
 * failure, nothing otherwise. A tag that names no variant is the
 * `unexpected_tag` error.
 */
export function decode(bytes: Uint8Array): Frame {
  const [tag, rest] = splitTag(bytes);
  switch (tag) {
    case DELIVERED:
      return "delivered";
    case NOT_LISTENING:
      return "not_listening";
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
