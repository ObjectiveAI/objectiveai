// An ask that names one path and nothing else.

import { z } from "zod";
import { text } from "../../../../../../../shared/containers/fuse/bytes.js";

/**
 * One path within what is served, as `/`-joined components with no
 * leading `/`; empty is the root. What a `stat`, a `list`, a `remove`
 * and a `mkdir` each carry: the path's bytes, as UTF-8, and nothing
 * else.
 */
export const Path = z.object({
  /** The path. */
  path: z.string(),
});
export type Path = z.infer<typeof Path>;

/** The ask as its bytes. */
export function encode(ask: Path): Uint8Array {
  return new TextEncoder().encode(ask.path);
}

/** An ask out of its bytes. */
export function decode(bytes: Uint8Array): Path {
  return { path: text(bytes) };
}
