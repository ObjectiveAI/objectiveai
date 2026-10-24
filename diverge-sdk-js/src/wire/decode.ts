// How a payload's bytes come back out as a type.
//
// Deliberately narrower than "parse": a decoder says which format a
// payload is in, and says it once, beside the type — so a payload
// has exactly one wire form and no caller can pick a different one
// by accident. The bytes handed to a decoder are the payload alone;
// the frame header has already been split off.

/** A decoder: the payload bytes to the value they carry. */
export type Decode<T> = (bytes: Uint8Array) => T;

/** Why a tagged payload could not be read. */
export type DecodeErrorKind = "empty" | "unexpected_tag" | "body";

/**
 * A payload with no tag byte at all, a tag that names no variant of
 * the type being read, or a body that would not parse as what the
 * tag promised.
 */
export class DecodeError extends Error {
  /** Which. */
  readonly kind: DecodeErrorKind;
  /** The tag read, for `unexpected_tag`. */
  readonly tag: number | undefined;

  constructor(kind: DecodeErrorKind, tag?: number, cause?: unknown) {
    super(
      kind === "empty"
        ? "the payload is empty"
        : kind === "unexpected_tag"
          ? `the tag ${tag} names no variant`
          : "the body would not parse",
      cause === undefined ? undefined : { cause },
    );
    this.name = "DecodeError";
    this.kind = kind;
    this.tag = tag;
  }
}

const DECODER = new TextDecoder("utf-8", { fatal: true });

/** The tag byte and the rest; an empty payload is the `empty` error. */
export function splitTag(bytes: Uint8Array): [number, Uint8Array] {
  if (bytes.length === 0) {
    throw new DecodeError("empty");
  }
  return [bytes[0] as number, bytes.subarray(1)];
}

/** `bytes` as UTF-8 text; bytes that are not are the `body` error. */
export function text(bytes: Uint8Array): string {
  try {
    return DECODER.decode(bytes);
  } catch (cause) {
    throw new DecodeError("body", undefined, cause);
  }
}

/** `bytes` as one JSON value; anything else is the `body` error. */
export function jsonOf(bytes: Uint8Array): unknown {
  try {
    return JSON.parse(text(bytes));
  } catch (cause) {
    if (cause instanceof DecodeError) {
      throw cause;
    }
    throw new DecodeError("body", undefined, cause);
  }
}
