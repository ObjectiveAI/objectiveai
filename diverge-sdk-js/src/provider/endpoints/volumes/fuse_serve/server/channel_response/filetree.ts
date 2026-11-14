// What answers a file tree channel on a FUSE serve scope.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../wire/decode.js";
import { Error } from "../../../../../../shared/error.js";
import * as filetree from "../../../../../../shared/filetree/response/frame.js";

/**
 * One frame of a served volume's file tree: the tree as the serve
 * sees it, first, then one frame per change, for as long as the scope
 * lives — or an error. One byte says which, `0` or `1`.
 */
export const Frame = z.union([
  /** The tree, or a change to it. Tag `0`. */
  z.strictObject({ filetree: filetree.Frame }),
  /** The tree could not be read. Tag `1`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

const FILETREE = 0;
const ERROR = 1;

/** A frame out of its payload. */
export function decode(bytes: Uint8Array): Frame {
  const [tag, rest] = splitTag(bytes);
  switch (tag) {
    case FILETREE:
      return { filetree: filetree.decode(rest) };
    case ERROR:
      return { error: Error.parse(jsonOf(rest)) };
    default:
      throw new DecodeError("unexpected_tag", tag);
  }
}
