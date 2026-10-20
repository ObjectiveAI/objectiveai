// A delete's answer.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../../wire/decode.js";
import { Error } from "../../../../../../../shared/error.js";

/**
 * A delete's answer: the template is gone, no template is the one
 * named, an agent was made from it, forbidden, or a failure.
 */
export const Frame = z.union([
  /** The template is gone, with its attachments. Tag `0`. */
  z.literal("deleted"),
  /** No template is the one named; nothing changed. Tag `1`. */
  z.literal("not_found"),
  /** An agent of the caller's was made from it; nothing changed. Tag `2`. */
  z.literal("in_use"),
  /** The account the request is served for holds no grant allowing it; nothing changed. Tag `3`. */
  z.literal("forbidden"),
  /** A failure. See `shared.error.Error` for why it says so little. Tag `4`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

const DELETED = 0;
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
    case DELETED:
      return "deleted";
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
