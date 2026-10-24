// A filetree's answer: a stream.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../wire/decode.js";
import { Error } from "../../../../../../shared/error.js";
import * as filetree from "../../../../../../shared/filetree/response/frame.js";

/**
 * A filetree's answer: one frame of the tree, no tool is the one
 * named, forbidden, or a failure. First one `snapshot`, then one
 * frame per change for the scope's life, until the client's cancel
 * or the tool's deletion; or the scope is exactly one `not_found`
 * or `forbidden`; or exactly one `error`, last. On the wire a frame
 * is postcard, as a filetree travels everywhere; as JSON it is the
 * tagged frame, `{"filetree":{"type":"snapshot",…}}`. Every
 * directory under a FUSE mount has `changes` `true`.
 */
export const Frame = z.union([
  /** One frame of the tree: the snapshot, or one change. Tag `0`. */
  z.strictObject({ filetree: filetree.Frame }),
  /** No tool is the one named; nothing was sent. Tag `1`. */
  z.literal("not_found"),
  /** The account the request is served for holds no grant allowing it; nothing changed. Tag `2`. */
  z.literal("forbidden"),
  /** A failure. See `shared.error.Error` for why it says so little. Tag `3`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

const FILETREE = 0;
const NOT_FOUND = 1;
const FORBIDDEN = 2;
const ERROR = 3;

/**
 * A frame out of its payload: the tag byte, then the variant's own
 * bytes — the tree's frame in postcard, nothing for a bare answer,
 * the error's JSON for a failure. A tag that names no variant is
 * the `unexpected_tag` error.
 */
export function decode(bytes: Uint8Array): Frame {
  const [tag, rest] = splitTag(bytes);
  switch (tag) {
    case FILETREE:
      return { filetree: filetree.decode(rest) };
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
