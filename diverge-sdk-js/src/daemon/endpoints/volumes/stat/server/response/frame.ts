// A stat's answer.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../wire/decode.js";
import { Error } from "../../../../../../shared/error.js";
import { Stat } from "./stat.js";

/**
 * A stat's answer: the two numbers, no volume is the one named, the
 * volume is held, forbidden, or a failure.
 */
export const Frame = z.union([
  /** The two numbers, as of the walk. Tag `0`. */
  z.strictObject({ stat: Stat }),
  /** No volume is the one named; nothing was walked. Tag `1`. */
  z.literal("not_found"),
  /** A running container has the volume, or another download, upload or transfer is on it; nothing was walked. Tag `2`. */
  z.literal("held"),
  /** The account the request is served for holds no grant allowing it; nothing changed. Tag `3`. */
  z.literal("forbidden"),
  /** A failure. See `shared.error.Error` for why it says so little. Tag `4`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

const STAT = 0;
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
    case STAT:
      return { stat: Stat.parse(jsonOf(rest)) };
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
