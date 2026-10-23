// An edit's answer.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../../wire/decode.js";
import { Error } from "../../../../../../../shared/error.js";

/**
 * An edit's answer: the credential is as the result states and this
 * is its new key's keyring entry, none names the identity, the new identity is
 * another's, forbidden, or a failure.
 */
export const Frame = z.union([
  /** The credential is as the request states, and this is the name of the entry in the daemon's host's keyring its new key is kept in — never the key. Tag `0`. */
  z.strictObject({ edited: z.string() }),
  /** No credential names the identity; nothing changed. Tag `1`. */
  z.literal("not_found"),
  /** The identity requested is another credential's; nothing changed. Tag `2`. */
  z.literal("in_use"),
  /** The account the request is served for holds no grant allowing it; nothing changed. Tag `3`. */
  z.literal("forbidden"),
  /** A failure. See `shared.error.Error` for why it says so little. Tag `4`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

const EDITED = 0;
const NOT_FOUND = 1;
const IN_USE = 2;
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
      return { edited: z.string().parse(jsonOf(rest)) };
    case NOT_FOUND:
      return "not_found";
    case IN_USE:
      return "in_use";
    case FORBIDDEN:
      return "forbidden";
    case ERROR:
      return { error: Error.parse(jsonOf(rest)) };
    default:
      throw new DecodeError("unexpected_tag", tag);
  }
}
