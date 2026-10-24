// An edit's answer.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../wire/decode.js";
import { Error } from "../../../../../../shared/error.js";

/**
 * An edit's answer: the account is as the result states; no account
 * is the one named; no such role; forbidden; or a failure. Exactly
 * one per scope, before the finish.
 */
export const Frame = z.union([
  /** The account is as the request states. Tag `0`. */
  z.literal("edited"),
  /** No account is the one named; nothing changed. Tag `1`. */
  z.literal("not_found"),
  /** A role named is none the daemon has; nothing changed. Tag `2`. */
  z.literal("no_role"),
  /** The account the request is served for holds no grant allowing it; nothing changed. Tag `3`. */
  z.literal("forbidden"),
  /** A failure. See `shared.error.Error` for why it says so little. Tag `4`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

const EDITED = 0;
const NOT_FOUND = 1;
const NO_ROLE = 2;
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
    case NO_ROLE:
      return "no_role";
    case FORBIDDEN:
      return "forbidden";
    case ERROR:
      return { error: Error.parse(jsonOf(rest)) };
    default:
      throw new DecodeError("unexpected_tag", tag);
  }
}
