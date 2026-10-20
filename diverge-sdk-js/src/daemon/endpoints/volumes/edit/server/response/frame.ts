// An edit's answer.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../wire/decode.js";
import { Error } from "../../../../../../shared/error.js";

/**
 * An edit's answer: the volume is as the request states, no volume is
 * the one named, the provider has no room, the volume holds more than
 * that, the volume is held, forbidden, or a failure.
 */
export const Frame = z.union([
  /** The volume is as the request states. Tag `0`. */
  z.literal("edited"),
  /** No volume is the one named; nothing changed. Tag `1`. */
  z.literal("not_found"),
  /** The provider cannot reserve that many bytes; nothing changed. Tag `2`. */
  z.literal("insufficient_capacity"),
  /** The volume holds more than the size asked for, so it cannot be shrunk to it; nothing changed. Tag `3`. */
  z.literal("content_too_large"),
  /** A running container has the volume, or another download, upload or transfer is on it; nothing changed. Tag `4`. */
  z.literal("held"),
  /** The account the request is served for holds no grant allowing it; nothing changed. Tag `5`. */
  z.literal("forbidden"),
  /** A failure. See `shared.error.Error` for why it says so little. Tag `6`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

const EDITED = 0;
const NOT_FOUND = 1;
const INSUFFICIENT_CAPACITY = 2;
const CONTENT_TOO_LARGE = 3;
const HELD = 4;
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
      return "edited";
    case NOT_FOUND:
      return "not_found";
    case INSUFFICIENT_CAPACITY:
      return "insufficient_capacity";
    case CONTENT_TOO_LARGE:
      return "content_too_large";
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
