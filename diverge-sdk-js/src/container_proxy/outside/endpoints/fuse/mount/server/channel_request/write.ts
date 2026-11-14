// The ask to write a piece of a file.

import { z } from "zod";
import { concat, takeU64, text, u64 } from "../../../../../../../shared/containers/fuse/bytes.js";
import * as prefixed from "../../../../../../../shared/containers/fuse/prefixed.js";

/**
 * A piece of a file to write: which file, at which byte, and the
 * bytes. The path behind its length, the offset as a `u64`,
 * big-endian, then every byte to write, to the end of the frame.
 */
export const Write = z.object({
  /** The file. */
  path: z.string(),
  /** Where the bytes go. */
  offset: z.number().int().nonnegative(),
  /** The bytes. */
  bytes: z.custom<Uint8Array>((value) => value instanceof Uint8Array),
});
export type Write = z.infer<typeof Write>;

/** The ask as its bytes. */
export function encode(ask: Write): Uint8Array {
  return concat(prefixed.put(new TextEncoder().encode(ask.path)), u64(ask.offset), ask.bytes);
}

/** An ask out of its bytes. The bytes to write are a view of `bytes`, not a copy. */
export function decode(bytes: Uint8Array): Write {
  const [path, rest] = prefixed.take(bytes);
  return { path: text(path), offset: takeU64(rest, 0), bytes: rest.subarray(8) };
}
