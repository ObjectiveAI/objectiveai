// A filetree's answer.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../wire/decode.js";
import { Error } from "../../../../../../shared/error.js";
import { Reader } from "../../../../../../wire/postcard.js";
import { Node, readNode } from "../../../../../../shared/filetree/response/node.js";

/**
 * A filetree's answer: the tree, no volume is the one named, a file
 * at the path, the volume held, forbidden, or a failure. One question
 * and one reply, so there is exactly one per scope, before the
 * finish. The tree is the entries of the directory the path names,
 * each with everything beneath it, as the `Node`s of a snapshot,
 * once: nothing follows, no change is ever reported, and every
 * directory in it has `changes` `false`; a caller that wants to know
 * what changed asks again, and compares. `not_found`,
 * `not_directory` and `held` are ANSWERS: nothing was walked and
 * nothing is retried. An `error` is the absence of an answer: the
 * daemon could not walk. On the wire the tree is postcard, as a
 * filetree travels everywhere; as JSON it is the tagged nodes,
 * `{"tree":[{"type":"directory",…}]}`.
 */
export const Frame = z.union([
  /** The tree: the entries beneath the path, each with everything beneath it. Tag `0`. */
  z.strictObject({ tree: z.array(Node) }),
  /** No volume is the one named, or nothing is at the path; nothing was walked. Tag `1`. */
  z.literal("not_found"),
  /** What is at the path is a file, which has no tree; nothing was walked. Tag `2`. */
  z.literal("not_directory"),
  /** A running container has the volume, or a download, an upload or a transfer is on it; nothing was walked. Tag `3`. */
  z.literal("held"),
  /** The account the request is served for holds no grant allowing it; nothing changed. Tag `4`. */
  z.literal("forbidden"),
  /** A failure. See `shared.error.Error` for why it says so little. Tag `5`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

const TREE = 0;
const NOT_FOUND = 1;
const NOT_DIRECTORY = 2;
const HELD = 3;
const FORBIDDEN = 4;
const ERROR = 5;

/**
 * A frame out of its payload: the tag byte, then the variant's own
 * bytes — the tree in postcard, nothing for a bare answer, the
 * error's JSON for a failure. A tag that names no variant is the
 * `unexpected_tag` error.
 */
export function decode(bytes: Uint8Array): Frame {
  const [tag, rest] = splitTag(bytes);
  switch (tag) {
    case TREE:
      return { tree: tree(rest) };
    case NOT_FOUND:
      return "not_found";
    case NOT_DIRECTORY:
      return "not_directory";
    case HELD:
      return "held";
    case FORBIDDEN:
      return "forbidden";
    case ERROR:
      return { error: Error.parse(jsonOf(rest)) };
    default:
      throw new DecodeError("unexpected_tag", tag);
  }
}

/** The tree out of its postcard bytes: a sequence of nodes, the whole payload consumed. */
function tree(bytes: Uint8Array): Node[] {
  const reader = new Reader(bytes);
  const nodes = reader.seq(readNode);
  reader.end();
  return nodes;
}
