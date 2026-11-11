// A detach's answer.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../wire/decode.js";
import { Error } from "../../../../../../shared/error.js";

/**
 * A detach's answer: the tool is detached, no tool is the one
 * named, no agent is, forbidden, or a failure.
 */
export const Frame = z.union([
  /** The tool is detached. Tag `0`. */
  z.literal("detached"),
  /** No tool is the one named; nothing changed. Tag `1`. */
  z.literal("no_tool"),
  /** No agent is the one named; nothing changed. Tag `2`. */
  z.literal("no_agent"),
  /** The account the request is served for holds no grant allowing it; nothing changed. Tag `3`. */
  z.literal("forbidden"),
  /** A failure. See `shared.error.Error` for why it says so little. Tag `4`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

const DETACHED = 0;
const NO_TOOL = 1;
const NO_AGENT = 2;
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
    case DETACHED:
      return "detached";
    case NO_TOOL:
      return "no_tool";
    case NO_AGENT:
      return "no_agent";
    case FORBIDDEN:
      return "forbidden";
    case ERROR:
      return { error: Error.parse(jsonOf(rest)) };
    default:
      throw new DecodeError("unexpected_tag", tag);
  }
}
