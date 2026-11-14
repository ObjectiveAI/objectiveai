// The ask to set a file's size.

import { z } from "zod";
import { concat, takeU64, text, u64 } from "../../../../../../../shared/containers/fuse/bytes.js";

/**
 * A file's new size: bytes past it are dropped, and a file shorter
 * than it is extended with zeros. The size as a `u64`, big-endian,
 * then the path, to the end of the frame.
 */
export const Truncate = z.object({
  /** The file. */
  path: z.string(),
  /** Its size from now on, in bytes. */
  size: z.number().int().nonnegative(),
});
export type Truncate = z.infer<typeof Truncate>;

/** The ask as its bytes. */
export function encode(ask: Truncate): Uint8Array {
  return concat(u64(ask.size), new TextEncoder().encode(ask.path));
}

/** An ask out of its bytes. */
export function decode(bytes: Uint8Array): Truncate {
  return { size: takeU64(bytes, 0), path: text(bytes.subarray(8)) };
}
