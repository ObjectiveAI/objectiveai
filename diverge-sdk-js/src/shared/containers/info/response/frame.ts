// What an info channel answers.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../wire/decode.js";
import { Error } from "../../../error.js";
import { Document } from "../document.js";

/**
 * The image's document, that it ships none, or why it could not be
 * read. Exactly one is sent on an `info` channel of an `images.read`
 * scope, and the finish follows it.
 */
export const Frame = z.union([
  /** The image's `/DIVERGE.md`, parsed. Tag `0`. */
  z.strictObject({ info: Document }),
  /** The image ships no `/DIVERGE.md`, or it is not a regular file. Tag `1`. */
  z.literal("missing"),
  /** A failure: the file is not a document, or the image could not be read. Tag `2`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

const INFO = 0;
const MISSING = 1;
const ERROR = 2;

/**
 * A frame out of its payload: the tag byte, then the variant's own
 * JSON, parsed as what the tag promised. A tag that names no variant
 * is the `unexpected_tag` error.
 */
export function decode(bytes: Uint8Array): Frame {
  const [tag, rest] = splitTag(bytes);
  switch (tag) {
    case INFO:
      return { info: Document.parse(jsonOf(rest)) };
    case MISSING:
      return "missing";
    case ERROR:
      return { error: Error.parse(jsonOf(rest)) };
    default:
      throw new DecodeError("unexpected_tag", tag);
  }
}
