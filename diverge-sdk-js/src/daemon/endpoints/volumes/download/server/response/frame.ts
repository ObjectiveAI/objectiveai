// A download's answer: one chunk, or one of the four answers.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../wire/decode.js";
import { Error } from "../../../../../../shared/error.js";
import * as download from "../../../../../../daemon/download/chunk.js";

/**
 * A download's answer, as JSON writes it: one chunk by its path and
 * its size, nothing is the one named, the volume held, forbidden, or
 * a failure. What the stream yields is the `Item`, the same frame
 * with the chunk's bytes; this is its printed form, and the schema
 * it is checked against.
 *
 * A download is a stream: zero or more chunks, each one piece of one
 * file with the file's path — see `daemon.download.Chunk` for the
 * order and what a path is relative to — then the finish; or exactly
 * one `not_found`, `held` or `forbidden`, then the finish; or chunks
 * and then exactly one `error`, then the finish. A directory with no
 * file in it is a scope that finishes with no response before it:
 * that answer, not a failure. A zero-byte file is one chunk, never
 * nothing. `not_found` and `held` are ANSWERS and nothing was sent; a
 * held volume is asked about again once it is free. An `error` is a
 * failure: the daemon could not read, or could not go on reading;
 * chunks sent before it precede it, none follow, and what was sent
 * is not whole.
 */
export const Frame = z.union([
  /** One piece of one file, by its path and its size. Tag `0`. */
  z.strictObject({ chunk: download.ChunkJson }),
  /** No volume is the one named, or nothing is at the path; nothing was sent. Tag `1`. */
  z.literal("not_found"),
  /** A running container has the volume, or another download, upload or transfer is on it; nothing was sent. Tag `2`. */
  z.literal("held"),
  /** The account the request is served for holds no grant allowing it; nothing was sent. Tag `3`. */
  z.literal("forbidden"),
  /** A failure. See `shared.error.Error` for why it says so little. Tag `4`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

/**
 * One response of the download as the stream yields it: the frame
 * with its chunk's bytes — the path, and the piece to append to the
 * pieces of the same file before it — or one of the answers.
 */
export type Item = { readonly chunk: download.Chunk } | "not_found" | "held" | "forbidden" | { readonly error: Error };

const CHUNK = 0;
const NOT_FOUND = 1;
const HELD = 2;
const FORBIDDEN = 3;
const ERROR = 4;

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
