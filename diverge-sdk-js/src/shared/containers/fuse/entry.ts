// One entry of a directory, and what kind of thing it is.

import { z } from "zod";
import { concat, text } from "./bytes.js";
import { FuseError } from "./error.js";
import * as prefixed from "./prefixed.js";

/** What is at a path: a regular file, or a directory. One byte on the wire, `0` or `1`. */
export const Kind = z.enum(["file", "directory"]);
export type Kind = z.infer<typeof Kind>;

/** The kind's byte. */
export function kindByte(kind: Kind): number {
  return kind === "file" ? 0 : 1;
}

/** The kind a byte names; any other byte is `unknown_kind`. */
export function kindOf(byte: number): Kind {
  switch (byte) {
    case 0:
      return "file";
    case 1:
      return "directory";
    default:
      throw new FuseError("unknown_kind", byte);
  }
}

/** One entry of a listing: its name within the directory, and its kind. */
export const Entry = z.object({
  /** The entry's name: one path component. */
  name: z.string(),
  /** What it is. */
  kind: Kind,
});
export type Entry = z.infer<typeof Entry>;

/** The entry as its bytes: the kind's byte, then the name behind its length. */
export function encode(entry: Entry): Uint8Array {
  return concat(Uint8Array.of(kindByte(entry.kind)), prefixed.put(new TextEncoder().encode(entry.name)));
}

/** The entry at the front of `bytes`, and what follows it. */
export function decode(bytes: Uint8Array): [Entry, Uint8Array] {
  if (bytes.length === 0) {
    throw new FuseError("truncated");
  }
  const kind = kindOf(bytes[0] as number);
  const [name, rest] = prefixed.take(bytes.subarray(1));
  return [{ name: text(name), kind }, rest];
}
