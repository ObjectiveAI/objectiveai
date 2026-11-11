// An identities list's answer.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../wire/decode.js";
import { Error } from "../../../../../../shared/error.js";
import { Identity } from "./identity.js";

/** The identities, by issuer then subject, each with its tags; forbidden; or a failure. Exactly one, then the finish. */
export const Frame = z.union([
  /** Every identity the filter lets through and the grants reach. Tag `0`. */
  z.strictObject({ listed: z.array(Identity) }),
  /** The account the request is served for holds no grant allowing it; nothing was read. Tag `1`. */
  z.literal("forbidden"),
  /** A failure: the identities could not be read. See `shared.error.Error` for why it says so little. Tag `2`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

const LISTED = 0;
const FORBIDDEN = 1;
const ERROR = 2;

/**
 * A frame out of its payload: the tag byte, then the variant's own
 * JSON, parsed as what the tag promised. A tag that names no variant
 * is the `unexpected_tag` error.
 */
export function decode(bytes: Uint8Array): Frame {
  const [tag, rest] = splitTag(bytes);
  switch (tag) {
    case LISTED:
      return { listed: z.array(Identity).parse(jsonOf(rest)) };
    case FORBIDDEN:
      return "forbidden";
    case ERROR:
      return { error: Error.parse(jsonOf(rest)) };
    default:
      throw new DecodeError("unexpected_tag", tag);
  }
}
