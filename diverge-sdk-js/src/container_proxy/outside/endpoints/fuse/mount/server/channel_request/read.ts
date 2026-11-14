// The ask for a piece of a file.

import { z } from "zod";
import { concat, takeU32, takeU64, text, u32, u64 } from "../../../../../../../shared/containers/fuse/bytes.js";
import * as prefixed from "../../../../../../../shared/containers/fuse/prefixed.js";

/**
 * A piece of a file: which file, from which byte, and how many. The
 * path behind its length, then the offset as a `u64` and the length
 * as a `u32`, both big-endian.
 */
export const Read = z.object({
  /** The file. */
  path: z.string(),
  /** The first byte wanted. */
  offset: z.number().int().nonnegative(),
  /** How many bytes are wanted, at most. */
  length: z.number().int().nonnegative(),
});
export type Read = z.infer<typeof Read>;

/** The ask as its bytes. */
export function encode(ask: Read): Uint8Array {
  return concat(prefixed.put(new TextEncoder().encode(ask.path)), u64(ask.offset), u32(ask.length));
}

/** An ask out of its bytes. */
export function decode(bytes: Uint8Array): Read {
  const [path, rest] = prefixed.take(bytes);
  return { path: text(path), offset: takeU64(rest, 0), length: takeU32(rest, 8) };
}
