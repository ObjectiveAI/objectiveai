// The attributes a setattr changes.

import { z } from "zod";
import { concat, takeU32, u32 } from "../bytes.js";
import { FuseError } from "../error.js";
import * as time from "../time.js";

/**
 * Which of a path's attributes to change, and to what: each present
 * is changed, each absent is left as it is. Thirty-seven bytes on the
 * wire: one byte of which are set — bit `0` the mode, `1` the uid,
 * `2` the gid, `3` the access time, `4` the modification time — then
 * every field in that order, zero where it is not set.
 */
export const Attrs = z.object({
  /** The mode bits. */
  mode: z.number().int().nonnegative().optional(),
  /** The owner. */
  uid: z.number().int().nonnegative().optional(),
  /** The group. */
  gid: z.number().int().nonnegative().optional(),
  /** The access time. */
  atime: time.Time.optional(),
  /** The modification time. */
  mtime: time.Time.optional(),
});
export type Attrs = z.infer<typeof Attrs>;

/** How many bytes the attributes are. */
export const ATTRS_LEN = 1 + 4 + 4 + 4 + 2 * time.TIME_LEN;

const MODE = 1 << 0;
const UID = 1 << 1;
const GID = 1 << 2;
const ATIME = 1 << 3;
const MTIME = 1 << 4;
const ZERO: time.Time = { secs: 0, nanos: 0 };

/** The attributes as their bytes. */
export function encode(attrs: Attrs): Uint8Array {
  const set =
    (attrs.mode === undefined ? 0 : MODE) |
    (attrs.uid === undefined ? 0 : UID) |
    (attrs.gid === undefined ? 0 : GID) |
    (attrs.atime === undefined ? 0 : ATIME) |
    (attrs.mtime === undefined ? 0 : MTIME);
  return concat(
    Uint8Array.of(set),
    u32(attrs.mode ?? 0),
    u32(attrs.uid ?? 0),
    u32(attrs.gid ?? 0),
    time.encode(attrs.atime ?? ZERO),
    time.encode(attrs.mtime ?? ZERO),
  );
}

/** The attributes at the front of `bytes`, and what follows them. */
export function decode(bytes: Uint8Array): [Attrs, Uint8Array] {
  if (bytes.length < ATTRS_LEN) {
    throw new FuseError("truncated");
  }
  const set = bytes[0] as number;
  const attrs: Attrs = {};
  if (set & MODE) {
    attrs.mode = takeU32(bytes, 1);
  }
  if (set & UID) {
    attrs.uid = takeU32(bytes, 5);
  }
  if (set & GID) {
    attrs.gid = takeU32(bytes, 9);
  }
  if (set & ATIME) {
    attrs.atime = time.decode(bytes, 13);
  }
  if (set & MTIME) {
    attrs.mtime = time.decode(bytes, 13 + time.TIME_LEN);
  }
  return [attrs, bytes.subarray(ATTRS_LEN)];
}
