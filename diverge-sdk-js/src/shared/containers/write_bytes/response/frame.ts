// A piece of the file being written.

/**
 * A piece of the file being written: bytes and nothing else — no
 * tag, because there is nothing here to discriminate. This is the
 * CONTENT, and content is the same content wherever a write happens.
 * Whether a channel carrying this can also carry a failure is the
 * endpoint's to decide, and the endpoints that allow one wrap this in
 * a frame of their own. It ends with a finish, and only with a
 * finish: there is no terminator in the bytes, and an empty piece is
 * a piece.
 */
export type Frame = Uint8Array;

/** The bytes, as they are. */
export function encode(frame: Frame): Uint8Array {
  return frame;
}

/** The bytes, as they are: every payload is a piece. */
export function decode(bytes: Uint8Array): Frame {
  return bytes;
}
