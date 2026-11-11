// What a provider's response frame carries for an image's info.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../wire/decode.js";
import { Error } from "../../../../../../shared/error.js";
import { Document } from "../../../../../../shared/containers/info/document.js";

/**
 * The image's document, that it has none, that the image was found
 * nowhere, or why it could not be read. Exactly one is sent, and the
 * finish follows it.
 */
export const Frame = z.union([
  /** The image's `/DIVERGE.md`, parsed. Tag `0`. */
  z.strictObject({ info: Document }),
  /** The image ships no `/DIVERGE.md`, or it is not a regular file. Tag `1`. */
  z.literal("missing"),
  /** The image is in no store and no registry the provider may ask. Tag `2`. */
  z.literal("no_source"),
  /** A failure: the file is not a document, or the image could not be read. Tag `3`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

const INFO = 0;
const MISSING = 1;
const NO_SOURCE = 2;
const ERROR = 3;

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
    case NO_SOURCE:
      return "no_source";
    case ERROR:
      return { error: Error.parse(jsonOf(rest)) };
    default:
      throw new DecodeError("unexpected_tag", tag);
  }
}
