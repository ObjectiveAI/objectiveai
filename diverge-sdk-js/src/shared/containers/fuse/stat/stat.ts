// What is at a path.

import { z } from "zod";
import { concat, takeU32, takeU64, u32, u64 } from "../bytes.js";
import { Kind, kindByte, kindOf } from "../entry.js";
import { FuseError } from "../error.js";
import * as time from "../time.js";

/**
 * What is at a path, as the volume records it: its kind, its size in
 * bytes, its mode, its owner and group, and the three times. Fifty-
 * seven bytes on the wire: the kind's byte, the size as a `u64`, the
 * mode, the uid and the gid as a `u32` each, then the access, the
 * modification and the change time.
 */
export const Stat = z.object({
  /** A file, or a directory. */
  kind: Kind,
  /** Its size in bytes. */
  size: z.number().int().nonnegative(),
  /** Its mode bits. */
  mode: z.number().int().nonnegative(),
  /** Its owner. */
  uid: z.number().int().nonnegative(),
  /** Its group. */
  gid: z.number().int().nonnegative(),
  /** When it was last read. */
  atime: time.Time,
  /** When its content last changed. */
  mtime: time.Time,
  /** When its attributes last changed. */
  ctime: time.Time,
});
export type Stat = z.infer<typeof Stat>;

/** How many bytes a stat is. */
export const STAT_LEN = 1 + 8 + 4 + 4 + 4 + 3 * time.TIME_LEN;

/** The stat as its bytes. */
export function encode(stat: Stat): Uint8Array {
  return concat(
    Uint8Array.of(kindByte(stat.kind)),
    u64(stat.size),
    u32(stat.mode),
    u32(stat.uid),
    u32(stat.gid),
    time.encode(stat.atime),
    time.encode(stat.mtime),
    time.encode(stat.ctime),
  );
}

/** The stat at the front of `bytes`; fewer bytes than a stat is are `truncated`. */
export function decode(bytes: Uint8Array): Stat {
  if (bytes.length < STAT_LEN) {
    throw new FuseError("truncated");
  }
  return {
    kind: kindOf(bytes[0] as number),
    size: takeU64(bytes, 1),
    mode: takeU32(bytes, 9),
    uid: takeU32(bytes, 13),
    gid: takeU32(bytes, 17),
    atime: time.decode(bytes, 21),
    mtime: time.decode(bytes, 21 + time.TIME_LEN),
    ctime: time.decode(bytes, 21 + 2 * time.TIME_LEN),
  };
}
