// A create's answer.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../../wire/decode.js";
import { Error } from "../../../../../../../shared/error.js";

/**
 * A create's answer: the template is kept, under its id; it was kept
 * already, under that same id; forbidden; or a failure. The two
 * first carry the id as a JSON string after the tag.
 */
export const Frame = z.union([
  /** The template is kept: its id, the hash of it. Tag `0`. */
  z.strictObject({ created: z.string() }),
  /** The template was kept already, by anyone: its id, the one the caller already has. Tag `1`. */
  z.strictObject({ exists: z.string() }),
  /** The account the request is served for holds no grant allowing it; nothing changed. Tag `2`. */
  z.literal("forbidden"),
  /** A failure. See `shared.error.Error` for why it says so little. Tag `3`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

const CREATED = 0;
const EXISTS = 1;
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
    case CREATED:
      return { created: z.string().parse(jsonOf(rest)) };
    case EXISTS:
      return { exists: z.string().parse(jsonOf(rest)) };
    case FORBIDDEN:
      return "forbidden";
    case ERROR:
      return { error: Error.parse(jsonOf(rest)) };
    default:
      throw new DecodeError("unexpected_tag", tag);
  }
}
