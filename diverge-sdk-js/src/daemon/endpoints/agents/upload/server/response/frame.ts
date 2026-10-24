// An upload's answer.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../wire/decode.js";
import { Error } from "../../../../../../shared/error.js";

/**
 * An upload's answer: the files are in place, no agent is the one
 * named, forbidden, or a failure. Sent once every content channel
 * has finished, so there is exactly one per scope, before the
 * finish. `not_found` is an ANSWER: nothing was asked for and
 * nothing landed. An `error` is the absence of an answer: a content
 * channel ended in an error, a path was never finished, the
 * container could not take the file; a file is at its path whole or
 * as it was, never the half between, and files that landed before
 * the failure stay.
 */
export const Frame = z.union([
  /** Every file is in place. Tag `0`. */
  z.literal("uploaded"),
  /** No agent is the one named; nothing landed. Tag `1`. */
  z.literal("not_found"),
  /** The account the request is served for holds no grant allowing it; nothing changed. Tag `2`. */
  z.literal("forbidden"),
  /** A failure. See `shared.error.Error` for why it says so little. Tag `3`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

const UPLOADED = 0;
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
    case UPLOADED:
      return "uploaded";
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
