// A get's answer.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../wire/decode.js";
import { Error } from "../../../../../../shared/error.js";
import { Mode } from "../../../../../../daemon/endpoints/postgres/mode.js";

/**
 * A get's answer: the mode, forbidden, or a failure. There is always
 * a mode: the daemon serves exactly one database.
 */
export const Frame = z.union([
  /** Which database the daemon serves: local, or remote with the URL, its password taken out. Tag `0`. */
  z.strictObject({ mode: Mode }),
  /** The account the request is served for holds no grant allowing it; nothing changed. Tag `1`. */
  z.literal("forbidden"),
  /** A failure. See `shared.error.Error` for why it says so little. Tag `2`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

const MODE = 0;
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
    case MODE:
      return { mode: Mode.parse(jsonOf(rest)) };
    case FORBIDDEN:
      return "forbidden";
    case ERROR:
      return { error: Error.parse(jsonOf(rest)) };
    default:
      throw new DecodeError("unexpected_tag", tag);
  }
}
