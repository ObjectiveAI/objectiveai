// A transfer's answer.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../wire/decode.js";
import { Error } from "../../../../../../shared/error.js";

/**
 * A transfer's answer: it landed, no volume is the one named, no
 * agent, tool, dependency tool or volume is the destination's, a
 * volume held, forbidden, or a failure. Sent once everything has
 * landed, so there is exactly one per scope, before the finish.
 * `not_found`, `no_destination` and `held` are ANSWERS: nothing
 * landed and nothing is retried. An `error` is the absence of an
 * answer: the daemon could not copy, or could not finish copying; a
 * file is at the destination whole or as it was, never the half
 * between, and files that landed before the failure stay.
 */
export const Frame = z.union([
  /** Everything landed. Tag `0`. */
  z.literal("transferred"),
  /** No volume is the one named, or nothing is at the path; nothing landed. Tag `1`. */
  z.literal("not_found"),
  /** The destination names an agent, a tool, a dependency tool or a volume that is none; nothing landed. Tag `2`. */
  z.literal("no_destination"),
  /** A volume at either end is held: a running container has it, or another download, upload or transfer is on it; nothing landed. Tag `3`. */
  z.literal("held"),
  /** The account the request is served for holds no grant allowing it; nothing changed. Tag `4`. */
  z.literal("forbidden"),
  /** A failure. See `shared.error.Error` for why it says so little. Tag `5`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

const TRANSFERRED = 0;
const NOT_FOUND = 1;
const NO_DESTINATION = 2;
const HELD = 3;
const FORBIDDEN = 4;
const ERROR = 5;

/**
 * A frame out of its payload: the tag byte, then the variant's own
 * JSON, parsed as what the tag promised — nothing for a bare
 * answer, the error's value for a failure. A tag that names no
 * variant is the `unexpected_tag` error.
 */
export function decode(bytes: Uint8Array): Frame {
  const [tag, rest] = splitTag(bytes);
  switch (tag) {
    case TRANSFERRED:
      return "transferred";
    case NOT_FOUND:
      return "not_found";
    case NO_DESTINATION:
      return "no_destination";
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
