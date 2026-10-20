// An edit's answer.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../wire/decode.js";
import { Error } from "../../../../../../shared/error.js";

/**
 * An edit's answer: the account is as the result states, with a new
 * key when the patch set a credential; no account is the one named;
 * no such role; the name or the identity is another's; the result
 * would leave the account alone; forbidden; or a failure. Exactly
 * one per scope, before the finish.
 */
export const Frame = z.union([
  /** The account is as the request states. The new key the daemon minted, when the request set a credential, answered here and never again; `null` otherwise. Tag `0`. */
  z.strictObject({ edited: z.string().nullable() }),
  /** No account is the one named; nothing changed. Tag `1`. */
  z.literal("not_found"),
  /** A role named is none the daemon has; nothing changed. Tag `2`. */
  z.literal("no_role"),
  /** The name or the identity requested is another account's; nothing changed. Tag `3`. */
  z.literal("in_use"),
  /** The change would leave the account with neither a name nor a credential, or take the name from one a container runs under, or the credential from one a client is connected as; nothing changed. Tag `4`. */
  z.literal("alone"),
  /** The account the request is served for holds no grant allowing it; nothing changed. Tag `5`. */
  z.literal("forbidden"),
  /** A failure. See `shared.error.Error` for why it says so little. Tag `6`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

const EDITED = 0;
const NOT_FOUND = 1;
const NO_ROLE = 2;
const IN_USE = 3;
const ALONE = 4;
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
    case EDITED:
      return { edited: z.string().nullable().parse(jsonOf(rest)) };
    case NOT_FOUND:
      return "not_found";
    case NO_ROLE:
      return "no_role";
    case IN_USE:
      return "in_use";
    case ALONE:
      return "alone";
    case FORBIDDEN:
      return "forbidden";
    case ERROR:
      return { error: Error.parse(jsonOf(rest)) };
    default:
      throw new DecodeError("unexpected_tag", tag);
  }
}
