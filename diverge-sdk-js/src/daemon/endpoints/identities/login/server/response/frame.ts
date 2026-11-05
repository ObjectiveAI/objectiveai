// A login's answers.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../wire/decode.js";
import { Error } from "../../../../../../shared/error.js";
import { Identity } from "../../../../../../shared/identity.js";

/** What a person needs to approve a login: never the device code. */
export const Pending = z.object({
  /** The code the person enters. */
  user_code: z.string(),
  /** Where the person enters it. */
  verification_uri: z.string(),
  /** Where the person need not enter it: the URI with the code in it, where the issuer gives one. */
  verification_uri_complete: z.string().optional(),
  /** The seconds the code is good for. */
  expires_in: z.number().int().nonnegative(),
});
export type Pending = z.infer<typeof Pending>;

/**
 * A login's answers. Begun: exactly one `pending`, then exactly one of
 * `signed_in`, `denied`, `expired`, `other_subject` or `error`, then
 * the finish — a finish with no outcome after `pending` is a cancel, or
 * the connection ending, and nothing was written. Refused: exactly one
 * of `held`, `forbidden` or `error`. No answer carries a token.
 */
export const Frame = z.union([
  /** The login begun: what a person needs to approve it. Tag `0`. */
  z.strictObject({ pending: Pending }),
  /** Approved as the identity named, and written on the daemon's host, replacing one already there. Tag `1`. */
  z.strictObject({ signed_in: Identity }),
  /** Another login of the identity, or a reader of it, holds its lock: nothing was asked of the issuer. Tag `2`. */
  z.literal("held"),
  /** The person denied the code; nothing was written. Tag `3`. */
  z.literal("denied"),
  /** The code expired before the person decided; nothing was written. Tag `4`. */
  z.literal("expired"),
  /** Approved as another identity, the one carried; nothing was written. Tag `5`. */
  z.strictObject({ other_subject: Identity }),
  /** The account the request is served for holds no grant allowing it. Tag `6`. */
  z.literal("forbidden"),
  /** A failure. See `shared.error.Error` for why it says so little. Tag `7`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

const PENDING = 0;
const SIGNED_IN = 1;
const HELD = 2;
const DENIED = 3;
const EXPIRED = 4;
const OTHER_SUBJECT = 5;
const FORBIDDEN = 6;
const ERROR = 7;

/**
 * A frame out of its payload: the tag byte, then the variant's own
 * JSON, parsed as what the tag promised. A tag that names no variant
 * is the `unexpected_tag` error.
 */
export function decode(bytes: Uint8Array): Frame {
  const [tag, rest] = splitTag(bytes);
  switch (tag) {
    case PENDING:
      return { pending: Pending.parse(jsonOf(rest)) };
    case SIGNED_IN:
      return { signed_in: Identity.parse(jsonOf(rest)) };
    case HELD:
      return "held";
    case DENIED:
      return "denied";
    case EXPIRED:
      return "expired";
    case OTHER_SUBJECT:
      return { other_subject: Identity.parse(jsonOf(rest)) };
    case FORBIDDEN:
      return "forbidden";
    case ERROR:
      return { error: Error.parse(jsonOf(rest)) };
    default:
      throw new DecodeError("unexpected_tag", tag);
  }
}
