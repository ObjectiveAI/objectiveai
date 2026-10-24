// How a type becomes a payload's bytes.
//
// A payload is never sent alone: it goes out behind a nine-byte
// header, and what an endpoint's `encode` hands back is the payload
// alone. Nearly every payload of the daemon's is one tag byte and
// then JSON, which is what these helpers write; the filetree's
// postcard has its own module.

/** An encoder: a value to the payload bytes that carry it. */
export type Encode<T> = (value: T) => Uint8Array;

const ENCODER = new TextEncoder();

/** `text` as UTF-8. */
export function utf8(text: string): Uint8Array {
  return ENCODER.encode(text);
}

/** `value` as compact JSON, in UTF-8. */
export function json(value: unknown): Uint8Array {
  return ENCODER.encode(JSON.stringify(value));
}

/**
 * One tag byte, then `body` — or the tag alone for a variant that
 * carries nothing. The shape of every tagged payload on the wire.
 */
export function tagged(tag: number, body?: Uint8Array): Uint8Array {
  if (body === undefined) {
    return Uint8Array.of(tag);
  }
  const out = new Uint8Array(1 + body.length);
  out[0] = tag;
  out.set(body, 1);
  return out;
}
