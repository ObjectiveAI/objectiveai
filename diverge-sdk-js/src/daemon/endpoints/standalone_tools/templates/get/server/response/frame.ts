// A get's answer.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../../wire/decode.js";
import { Error } from "../../../../../../../shared/error.js";
import { Listed } from "../../../../../../../daemon/endpoints/standalone_tools/templates/list/server/response/listed.js";

/**
 * A get's answer: the template as a list would report it, no
 * template is the one named, forbidden, or a failure.
 */
export const Frame = z.union([
  /** The template, as the list reports it. Tag `0`. */
  z.strictObject({ found: Listed }),
  /** No template is the one named. Tag `1`. */
  z.literal("not_found"),
  /** The account the request is served for holds no grant allowing it; nothing changed. Tag `2`. */
  z.literal("forbidden"),
  /** A failure. See `shared.error.Error` for why it says so little. Tag `3`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

const FOUND = 0;
const NOT_FOUND = 1;
const FORBIDDEN = 2;
const ERROR = 3;

/**
 * A frame out of its payload: the tag byte, then the variant's own
 * JSON, parsed as what the tag promised — nothing for a bare
 * answer, the error's value for a failure. A tag that names no
 * variant is the `unexpected_tag` error.
 */
export function decode(bytes: Uint8Array): Frame {
  const [tag, rest] = splitTag(bytes);
  switch (tag) {
    case FOUND:
      return { found: Listed.parse(jsonOf(rest)) };
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
