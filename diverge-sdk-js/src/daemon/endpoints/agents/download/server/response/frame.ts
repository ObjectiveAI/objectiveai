// A download's answer: one chunk, or one of the three answers.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../wire/decode.js";
import { Error } from "../../../../../../shared/error.js";
import * as download from "../../../../../../daemon/download/chunk.js";

/**
 * A download's answer, as JSON writes it: one chunk by its path and
 * its size, nothing is the one named, forbidden, or a failure. What
 * the stream yields is the `Item`, the same frame with the chunk's
 * bytes; this is its printed form, and the schema it is checked
 * against. A download is a stream: zero or more chunks then the
 * finish; or exactly one `not_found` or `forbidden`, then the
 * finish; or chunks and then exactly one `error`, then the finish.
 * A directory with no file in it is a scope that finishes with no
 * response before it. A zero-byte file is one chunk, never nothing.
 */
export const Frame = z.union([
  /** One piece of one file, by its path and its size. Tag `0`. */
  z.strictObject({ chunk: download.ChunkJson }),
  /** No agent is the one named, or nothing is at the path; nothing was sent. Tag `1`. */
  z.literal("not_found"),
  /** The account the request is served for holds no grant allowing it; nothing was sent. Tag `2`. */
  z.literal("forbidden"),
  /** A failure. See `shared.error.Error` for why it says so little. Tag `3`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

/**
 * One response of the download as the stream yields it: the frame
 * with its chunk's bytes — the path, and the piece to append to the
 * pieces of the same file before it — or one of the answers.
 */
export type Item = { readonly chunk: download.Chunk } | "not_found" | "forbidden" | { readonly error: Error };

const CHUNK = 0;
const NOT_FOUND = 1;
const FORBIDDEN = 2;
const ERROR = 3;

/**
 * An item out of its payload: the tag byte, then the chunk as
 * `daemon.download` lays it out, nothing for a bare answer, the
 * error's JSON for a failure. A tag that names no variant is the
 * `unexpected_tag` error.
 */
export function decode(bytes: Uint8Array): Item {
  const [tag, rest] = splitTag(bytes);
  switch (tag) {
    case CHUNK:
      return { chunk: download.decode(rest) };
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

/** An item as JSON writes it: a chunk by its path and its size, never its bytes. */
export function json(item: Item): Frame {
  if (typeof item === "string") {
    return item;
  }
  if ("chunk" in item) {
    return { chunk: { path: item.chunk.path, bytes: item.chunk.body.length } };
  }
  return item;
}
