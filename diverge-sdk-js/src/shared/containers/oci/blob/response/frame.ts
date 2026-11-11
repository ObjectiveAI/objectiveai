// One piece of the blob being fetched.

/**
 * A piece of the blob's bytes, verbatim — no header, no tag: the ask
 * named ONE blob. Every frame appends to what arrived before, and the
 * channel's finish is what says the blob is whole; no frame says
 * "last one". A short blob is caught by its digest, never by a count.
 */
export type Frame = Uint8Array;

/** The bytes, as they are: every payload is a piece. */
export function decode(bytes: Uint8Array): Frame {
  return bytes;
}
