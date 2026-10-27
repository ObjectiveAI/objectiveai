// An image info's answer.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../wire/decode.js";
import { Error } from "../../../../../../shared/error.js";
import { Document } from "../../../../../../shared/containers/info/document.js";

/**
 * An image info's answer: the document, that the image ships none,
 * that nothing could be asked, forbidden, or a failure. Nothing is
 * retried.
 */
export const Frame = z.union([
  /** The image's `/DIVERGE.md`, parsed. Tag `0`. */
  z.strictObject({ info: Document }),
  /** The image ships no `/DIVERGE.md`. Tag `1`. */
  z.literal("missing"),
  /** No container of the image runs, and no connected provider can obtain it: nothing was asked. Tag `2`. */
  z.literal("no_source"),
  /** The account the request is served for holds no grant allowing it; nothing was asked. Tag `3`. */
  z.literal("forbidden"),
  /** A failure. See `shared.error.Error` for why it says so little. Tag `4`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

const INFO = 0;
const MISSING = 1;
const NO_SOURCE = 2;
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
    case INFO:
      return { info: Document.parse(jsonOf(rest)) };
    case MISSING:
      return "missing";
    case NO_SOURCE:
      return "no_source";
    case FORBIDDEN:
      return "forbidden";
    case ERROR:
      return { error: Error.parse(jsonOf(rest)) };
    default:
      throw new DecodeError("unexpected_tag", tag);
  }
}
