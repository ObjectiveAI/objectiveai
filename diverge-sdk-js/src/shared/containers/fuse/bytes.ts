// The fixed-width fields every FUSE frame is made of.
//
// Every integer is big-endian. A `u64` is a `number` here, as it is
// everywhere in this package: exact up to 2^53, which no file's
// offset or size reaches.

import { FuseError } from "./error.js";

const DECODER = new TextDecoder("utf-8", { fatal: true });

/** `parts` as one run of bytes, in order. */
export function concat(...parts: Uint8Array[]): Uint8Array {
  const out = new Uint8Array(parts.reduce((length, part) => length + part.length, 0));
  let at = 0;
  for (const part of parts) {
    out.set(part, at);
    at += part.length;
  }
  return out;
}

/** `value` as four bytes. */
export function u32(value: number): Uint8Array {
  const out = new Uint8Array(4);
  new DataView(out.buffer).setUint32(0, value, false);
  return out;
}

/** `value` as eight bytes. */
export function u64(value: number): Uint8Array {
  const out = new Uint8Array(8);
  new DataView(out.buffer).setBigUint64(0, BigInt(value), false);
  return out;
}

/** The `u32` at `at`; fewer than four bytes there is `truncated`. */
export function takeU32(bytes: Uint8Array, at: number): number {
  if (bytes.length < at + 4) {
    throw new FuseError("truncated");
  }
  return new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength).getUint32(at, false);
}

/** The `u64` at `at`; fewer than eight bytes there is `truncated`. */
export function takeU64(bytes: Uint8Array, at: number): number {
  if (bytes.length < at + 8) {
    throw new FuseError("truncated");
  }
  return Number(new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength).getBigUint64(at, false));
}

/** `bytes` as UTF-8 text; bytes that are not are `utf8`. */
export function text(bytes: Uint8Array): string {
  try {
    return DECODER.decode(bytes);
  } catch {
    throw new FuseError("utf8");
  }
}
