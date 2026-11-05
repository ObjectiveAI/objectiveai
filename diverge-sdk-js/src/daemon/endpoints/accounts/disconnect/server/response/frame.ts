// A disconnect's answer.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../wire/decode.js";
import { Error } from "../../../../../../shared/error.js";

/**
 * A disconnect's answer: how many connections a client had as the
 * account, or the administrator, were told to close — `0` where none was
 * open — no account is the one named, forbidden, or a failure. Exactly
 * one per scope, before the finish; the connections are not waited for.
 */
export const Frame = z.union([
  /** How many connections were told to close. Tag `0`. */
  z.strictObject({ disconnected: z.number().int().nonnegative() }),
  /** No account, and no administrator, is the one named; nothing changed. Tag `1`. */
  z.literal("not_found"),
  /** The account the request is served for holds no grant allowing it; nothing changed. Tag `2`. */
  z.literal("forbidden"),
  /** A failure. See `shared.error.Error` for why it says so little. Tag `3`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

const DISCONNECTED = 0;
const NOT_FOUND = 1;
const FORBIDDEN = 2;
const ERROR = 3;

/**
 * A frame out of its payload: the tag byte, then the variant's own JSON,
 * parsed as what the tag promised. A tag that names no variant is the
 * `unexpected_tag` error.
 */
export function decode(bytes: Uint8Array): Frame {
  const [tag, rest] = splitTag(bytes);
  switch (tag) {
    case DISCONNECTED:
      return { disconnected: z.number().int().nonnegative().parse(jsonOf(rest)) };
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
