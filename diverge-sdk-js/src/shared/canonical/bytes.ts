// A value's canonical bytes: what is hashed.

import { sortObjectKeys } from "./sort.js";

const ENCODER = new TextEncoder();

/**
 * The value as compact JSON with every object key sorted at every
 * depth: the bytes every id that is a hash is the SHA-256 of. The
 * value is one already stripped of its absent members, as a parsed
 * schema's output is; `undefined` members are not written, as serde
 * skips them.
 */
export function bytes(value: unknown): Uint8Array {
  return ENCODER.encode(JSON.stringify(sortObjectKeys(value)));
}

/** The lowercase hexadecimal SHA-256 of `value`'s canonical bytes. */
export async function sha256(value: unknown): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", bytes(value) as Uint8Array<ArrayBuffer>);
  return Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, "0")).join("");
}
