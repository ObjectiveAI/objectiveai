// One piece of one file, and which file.

import { z } from "zod";
import { DecodeError, jsonOf } from "../../wire/decode.js";

/**
 * A chunk as JSON writes it: the path, and how many bytes, never
 * the bytes — what a client that prints what the daemon answered
 * sees. The bytes themselves are the wire's, read by `decode`.
 */
export const ChunkJson = z.object({
  /** The file's path, as components. */
  path: z.array(z.string()),
  /** How many bytes the piece carried. */
  bytes: z.number().int().nonnegative(),
});
export type ChunkJson = z.infer<typeof ChunkJson>;

/**
 * One piece of one file in a download, with the file's path. The
 * chunks of a file carry the empty path: the file is what was asked
 * for. The chunks of a file in a directory carry its path within
 * that directory, as components. Every chunk carries its path, the
 * file's chunks are consecutive and in order, and files come in
 * bytewise order of their paths. A zero-byte file is exactly one
 * chunk with an empty body.
 */
export interface Chunk {
  /** The file, relative to what was asked for: empty for the file itself. */
  readonly path: string[];
  /** The piece: at most `CHUNK_SIZE` bytes, appended to the pieces before it. */
  readonly body: Uint8Array;
}

/**
 * A chunk out of its wire bytes: a `u32`, big-endian, then that many
 * bytes of JSON — the path as an array of strings — then the body,
 * raw to the end of the payload. Fewer bytes than the length counts,
 * or a path that is not an array of strings, is the `DecodeError`.
 */
export function decode(bytes: Uint8Array): Chunk {
  if (bytes.length < 4) {
    throw new DecodeError("body");
  }
  const length = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength).getUint32(0, false);
  if (bytes.length < 4 + length) {
    throw new DecodeError("body");
  }
  const parsed = z.array(z.string()).safeParse(jsonOf(bytes.subarray(4, 4 + length)));
  if (!parsed.success) {
    throw new DecodeError("body", undefined, parsed.error);
  }
  return { path: parsed.data, body: bytes.subarray(4 + length) };
}
