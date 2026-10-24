// An edit's answer.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../wire/decode.js";
import { Error } from "../../../../../../shared/error.js";

/**
 * An edit's answer: the agent is as the request states, no agent is
 * the one named, a running agent refused a change it cannot take,
 * the new name is held, forbidden, or a failure.
 */
export const Frame = z.union([
  /** The agent is as the request states. Tag `0`. */
  z.literal("edited"),
  /** No agent is the one named; nothing changed. Tag `1`. */
  z.literal("not_found"),
  /** The agent runs, and the patch changes more than appending FUSE mounts; nothing changed. Tag `2`. */
  z.literal("active"),
  /** The name the patch gives is another agent's; nothing changed. Tag `3`. */
  z.literal("in_use"),
  /** The account the request is served for holds no grant allowing it; nothing changed. Tag `4`. */
  z.literal("forbidden"),
  /** A failure. See `shared.error.Error` for why it says so little. Tag `5`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

const EDITED = 0;
const NOT_FOUND = 1;
const ACTIVE = 2;
const IN_USE = 3;
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
    case EDITED:
      return "edited";
    case NOT_FOUND:
      return "not_found";
    case ACTIVE:
      return "active";
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
