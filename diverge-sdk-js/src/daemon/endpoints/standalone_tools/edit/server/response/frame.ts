// An edit's answer.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../wire/decode.js";
import { Error } from "../../../../../../shared/error.js";

/**
 * An edit's answer: the tool is as the request states, no tool is
 * the one named, a running tool refused a change it cannot take, a
 * connected tool refused one that is not its name, the new name is
 * held, no account is the one named, forbidden, or a failure.
 */
export const Frame = z.union([
  /** The tool is as the request states. Tag `0`. */
  z.literal("edited"),
  /** No tool is the one named; nothing changed. Tag `1`. */
  z.literal("not_found"),
  /** The tool runs, and the patch changes more than appending FUSE mounts; nothing changed. Tag `2`. */
  z.literal("active"),
  /** The tool is another daemon's, and the patch changes more than its name; nothing changed. Tag `3`. */
  z.literal("not_owned"),
  /** The name the patch gives is another tool's; nothing changed. Tag `4`. */
  z.literal("in_use"),
  /** No named account is the one the patch gives; nothing changed. Tag `5`. */
  z.literal("no_account"),
  /** The account the request is served for holds no grant allowing it; nothing changed. Tag `6`. */
  z.literal("forbidden"),
  /** A failure. See `shared.error.Error` for why it says so little. Tag `7`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

const EDITED = 0;
const NOT_FOUND = 1;
const ACTIVE = 2;
const NOT_OWNED = 3;
const IN_USE = 4;
const NO_ACCOUNT = 5;
const FORBIDDEN = 6;
const ERROR = 7;

/**
 * A frame out of its payload: the tag byte, then the variant's own
 * JSON, parsed as what the tag promised — nothing for a bare
 * answer, the error's value for a failure. A tag that names no
 * variant is the `unexpected_tag` error.
 */
export function decode(bytes: Uint8Array): Frame {
  const [tag, rest] = splitTag(bytes);
  switch (tag) {
    case EDITED:
      return "edited";
    case NOT_FOUND:
      return "not_found";
    case ACTIVE:
      return "active";
    case NOT_OWNED:
      return "not_owned";
    case IN_USE:
      return "in_use";
    case NO_ACCOUNT:
      return "no_account";
    case FORBIDDEN:
      return "forbidden";
    case ERROR:
      return { error: Error.parse(jsonOf(rest)) };
    default:
      throw new DecodeError("unexpected_tag", tag);
  }
}
