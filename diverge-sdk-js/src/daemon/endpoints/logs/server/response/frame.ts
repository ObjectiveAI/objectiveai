// A log's answer: a stream.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../wire/decode.js";
import { Error } from "../../../../../shared/error.js";
import { Entry } from "./entry.js";

/**
 * A log's answer: one item, forbidden, or a failure. Zero or more
 * items, each in log order, then the finish — after the client's
 * cancel, the count met, the span exhausted, or the subject's
 * deletion; or exactly one `error`, last; or the scope is exactly
 * one `forbidden`. A finish with nothing before it is a log in which
 * nothing matched. Not a listing: nothing is ever changed or removed.
 */
export const Frame = z.union([
  /** One item of the log, with its place and its time. Tag `0`. */
  z.strictObject({ item: Entry }),
  /** The account the request is served for holds no grant allowing it; nothing changed. Tag `1`. */
  z.literal("forbidden"),
  /** A failure. See `shared.error.Error` for why it says so little. Tag `2`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

const ITEM = 0;
const FORBIDDEN = 1;
const ERROR = 2;

/**
 * A frame out of its payload: the tag byte, then the variant's own
 * JSON, parsed as what the tag promised — nothing for a bare
 * answer, the error's value for a failure. A tag that names no
 * variant is the `unexpected_tag` error.
 */
export function decode(bytes: Uint8Array): Frame {
  const [tag, rest] = splitTag(bytes);
  switch (tag) {
    case ITEM:
      return { item: Entry.parse(jsonOf(rest)) };
    case FORBIDDEN:
      return "forbidden";
    case ERROR:
      return { error: Error.parse(jsonOf(rest)) };
    default:
      throw new DecodeError("unexpected_tag", tag);
  }
}
