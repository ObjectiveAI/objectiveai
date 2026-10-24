// An upload's answer.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../wire/decode.js";
import { Error } from "../../../../../../shared/error.js";

/**
 * An upload's answer: the files are in place, no volume is the one
 * named, the volume is held, forbidden, or a failure. Sent once every
 * content channel has finished, so there is exactly one per scope,
 * before the finish. `not_found` and `held` are ANSWERS: nothing was
 * asked for and nothing landed. An `error` is the absence of an
 * answer: a content channel ended in an error, a path was never
 * finished, the provider could not take the file, whatever the daemon
 * knows; a file is at its path whole or as it was, never the half
 * between, and files that landed before the failure stay.
 */
export const Frame = z.union([
  /** Every file is in place. Tag `0`. */
  z.literal("uploaded"),
  /** No volume is the one named; nothing landed. Tag `1`. */
  z.literal("not_found"),
  /** A running container has the volume, or another download, upload or transfer is on it; nothing landed. Tag `2`. */
  z.literal("held"),
  /** The account the request is served for holds no grant allowing it; nothing changed. Tag `3`. */
  z.literal("forbidden"),
  /** A failure. See `shared.error.Error` for why it says so little. Tag `4`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

const UPLOADED = 0;
const NOT_FOUND = 1;
const HELD = 2;
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
    case UPLOADED:
      return "uploaded";
    case NOT_FOUND:
      return "not_found";
    case HELD:
      return "held";
    case FORBIDDEN:
      return "forbidden";
    case ERROR:
      return { error: Error.parse(jsonOf(rest)) };
    default:
      throw new DecodeError("unexpected_tag", tag);
  }
}
