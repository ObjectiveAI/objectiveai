// The ask to move one path to another.

import { z } from "zod";
import { concat, text } from "../../../../../../../shared/containers/fuse/bytes.js";
import * as prefixed from "../../../../../../../shared/containers/fuse/prefixed.js";

/**
 * A move: from one path to another, within what is served. The first
 * path behind its length, then the second, to the end of the frame.
 */
export const Rename = z.object({
  /** The path as it is. */
  from: z.string(),
  /** The path it is to be. */
  to: z.string(),
});
export type Rename = z.infer<typeof Rename>;

/** The ask as its bytes. */
export function encode(ask: Rename): Uint8Array {
  return concat(prefixed.put(new TextEncoder().encode(ask.from)), new TextEncoder().encode(ask.to));
}

/** An ask out of its bytes. */
export function decode(bytes: Uint8Array): Rename {
  const [from, to] = prefixed.take(bytes);
  return { from: text(from), to: text(to) };
}
