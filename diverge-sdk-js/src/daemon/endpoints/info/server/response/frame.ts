// An info's answer.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../wire/decode.js";
import { Error } from "../../../../../shared/error.js";
import { Info } from "./info.js";

/**
 * An info's answer: what the daemon is, forbidden, or a failure.
 * Served here, it is always `info`; `forbidden` and `error` are a
 * relay's — one the account may not use, or one that failed.
 */
export const Frame = z.union([
  /** What the daemon is: its version. Tag `0`. */
  z.strictObject({ info: Info }),
  /** The account the request is served for may not use the relay it named; nothing was asked. Tag `1`. */
  z.literal("forbidden"),
  /** A failure. See `shared.error.Error` for why it says so little. Tag `2`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

const INFO = 0;
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
    case INFO:
      return { info: Info.parse(jsonOf(rest)) };
    case FORBIDDEN:
      return "forbidden";
    case ERROR:
      return { error: Error.parse(jsonOf(rest)) };
    default:
      throw new DecodeError("unexpected_tag", tag);
  }
}
