// A create's answer.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../wire/decode.js";
import { Error } from "../../../../../../shared/error.js";

/**
 * A create's answer: the account exists, one with the identity exists
 * already, no such role, forbidden, or a failure. Exactly one per scope, before the finish. `exists`
 * and `no_role` are ANSWERS: the daemon looked, and nothing was
 * created; `error` is the absence of an answer.
 */
export const Frame = z.union([
  /** The account exists, with its roles. Tag `0`. */
  z.literal("created"),
  /** The identity is an account's already; nothing was created. Tag `1`. */
  z.literal("exists"),
  /** A role named is none the daemon has; nothing was created. Tag `2`. */
  z.literal("no_role"),
  /** The account the request is served for holds no grant allowing it; nothing changed. Tag `3`. */
  z.literal("forbidden"),
  /** A failure. See `shared.error.Error` for why it says so little. Tag `4`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

const CREATED = 0;
const EXISTS = 1;
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
    case CREATED:
      return "created";
    case EXISTS:
      return "exists";
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
