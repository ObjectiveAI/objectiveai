// A attach's answer.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../wire/decode.js";
import { Error } from "../../../../../../shared/error.js";

/**
 * An attach's answer: the tool is attached, no tool is the one
 * named, no agent template is, forbidden, or a failure.
 */
export const Frame = z.union([
  /** The tool is attached. Tag `0`. */
  z.literal("attached"),
  /** No tool is the one named; nothing changed. Tag `1`. */
  z.literal("no_tool"),
  /** No agent template is the one named; nothing changed. Tag `2`. */
  z.literal("no_template"),
  /** The account the request is served for holds no grant allowing it; nothing changed. Tag `3`. */
  z.literal("forbidden"),
  /** A failure. See `shared.error.Error` for why it says so little. Tag `4`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

const ATTACHED = 0;
const NO_TOOL = 1;
const NO_TEMPLATE = 2;
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
    case ATTACHED:
      return "attached";
    case NO_TOOL:
      return "no_tool";
    case NO_TEMPLATE:
      return "no_template";
    case FORBIDDEN:
      return "forbidden";
    case ERROR:
      return { error: Error.parse(jsonOf(rest)) };
    default:
      throw new DecodeError("unexpected_tag", tag);
  }
}
