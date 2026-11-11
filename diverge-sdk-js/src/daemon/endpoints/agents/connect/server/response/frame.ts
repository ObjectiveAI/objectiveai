// An agents connect's one answer.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../wire/decode.js";
import { Error } from "../../../../../../shared/error.js";

/**
 * An agents connect's answer: connected, no agent is the one named,
 * forbidden, or a failure. Exactly one, first, and nothing follows it
 * on the scope for its life: what follows comes on the channels the
 * client opens. The finish is the disconnect's answer.
 */
export const Frame = z.union([
  /** The agent is the one named, and the grants reach it. Carries nothing. Tag `0`. */
  z.literal("connected"),
  /** No agent is the one named. Tag `1`. */
  z.literal("not_found"),
  /** The account the request is served for holds no grant allowing it; nothing changed. Tag `2`. */
  z.literal("forbidden"),
  /** A failure. See `shared.error.Error` for why it says so little. Tag `3`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

const CONNECTED = 0;
const NOT_FOUND = 1;
const FORBIDDEN = 2;
const ERROR = 3;

/**
 * A frame out of its payload: the tag byte, then — for the error
 * alone — the error's value. A tag that names no variant is the
 * `unexpected_tag` error.
 */
export function decode(bytes: Uint8Array): Frame {
  const [tag, rest] = splitTag(bytes);
  switch (tag) {
    case CONNECTED:
      return "connected";
    case NOT_FOUND:
      return "not_found";
    case FORBIDDEN:
      return "forbidden";
    case ERROR:
      return { error: Error.parse(jsonOf(rest)) };
    default:
      throw new DecodeError("unexpected_tag", tag);
  }
}
