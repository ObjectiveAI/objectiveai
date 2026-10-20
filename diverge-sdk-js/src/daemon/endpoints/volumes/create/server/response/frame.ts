// A create's answer.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../wire/decode.js";
import { Error } from "../../../../../../shared/error.js";

/**
 * A create's answer: the volume exists, no provider is the one named,
 * the name is taken, the provider has no room, the provider keeps no
 * such volume, forbidden, or a failure. Only the error carries
 * anything after its tag.
 */
export const Frame = z.union([
  /** The volume exists, and a list shows it. Tag `0`. */
  z.literal("created"),
  /** No provider is the one named; nothing exists. Tag `1`. */
  z.literal("no_provider"),
  /** The name is a volume's on that provider already; nothing changed. Tag `2`. */
  z.literal("exists"),
  /** The provider cannot reserve that many bytes; nothing exists. Tag `3`. */
  z.literal("insufficient_capacity"),
  /** The provider's capabilities keep no volume, or none of the mode asked; nothing exists, and the provider was not asked. Tag `4`. */
  z.literal("unsupported"),
  /** The account the request is served for holds no grant allowing it; nothing changed. Tag `5`. */
  z.literal("forbidden"),
  /** A failure. See `shared.error.Error` for why it says so little. Tag `6`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

const CREATED = 0;
const NO_PROVIDER = 1;
const EXISTS = 2;
const INSUFFICIENT_CAPACITY = 3;
const UNSUPPORTED = 4;
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
    case CREATED:
      return "created";
    case NO_PROVIDER:
      return "no_provider";
    case EXISTS:
      return "exists";
    case INSUFFICIENT_CAPACITY:
      return "insufficient_capacity";
    case UNSUPPORTED:
      return "unsupported";
    case FORBIDDEN:
      return "forbidden";
    case ERROR:
      return { error: Error.parse(jsonOf(rest)) };
    default:
      throw new DecodeError("unexpected_tag", tag);
  }
}
