// The manifest, whole.

import { DecodeError, text } from "../../../../../wire/decode.js";

/**
 * A manifest's media type and its bytes, in one frame. The media type
 * rides along because it is not recoverable from the bytes; the bytes
 * are the manifest verbatim — the digest is of these bytes, so
 * nothing may re-serialize them.
 *
 * On the wire, two parts: the media type's byte length as two bytes,
 * big-endian, then the media type, then everything after it the
 * manifest's bytes.
 */
export interface Frame {
  /** The manifest's media type — `application/vnd.oci.image.manifest.v1+json`, an index's, or the Docker equivalents. */
  readonly media_type: string;
  /** The manifest's bytes, verbatim. */
  readonly body: Uint8Array;
}

/** The bytes the media type's length occupies. */
const MEDIA_TYPE_LEN = 2;

/**
 * A frame out of its payload: the length, the media type, the rest.
 * Too short for the length, too short for the media type it names, or
 * a media type that is not UTF-8 is the `body` error.
 */
export function decode(bytes: Uint8Array): Frame {
  if (bytes.length < MEDIA_TYPE_LEN) {
    throw new DecodeError("body");
  }
  const length = ((bytes[0] as number) << 8) | (bytes[1] as number);
  const rest = bytes.subarray(MEDIA_TYPE_LEN);
  if (rest.length < length) {
    throw new DecodeError("body");
  }
  return { media_type: text(rest.subarray(0, length)), body: rest.subarray(length) };
}
