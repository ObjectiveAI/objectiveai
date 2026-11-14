// What the daemon asks a client for during a push.

import { DecodeError, jsonOf, splitTag } from "../../../../../../wire/decode.js";
import * as blob from "../../../../../../shared/containers/oci/blob/request/request.js";
import * as manifest from "../../../../../../shared/containers/oci/manifest/request/request.js";
import * as prove from "../../../../../../shared/containers/oci/prove/request/request.js";

/**
 * What the daemon asks a client for while a provider takes a pushed
 * image from it: the provider push's own asks, carried as they came.
 * One byte says which, and the rest is the ask's own JSON.
 *
 * | tag | asks for | answered with |
 * |-----|----------|---------------|
 * | `0` | `oci_manifest`: a manifest of the image, by digest | at most one frame, the media type and the bytes; the empty finish is not held |
 * | `1` | `oci_blob`: a blob of the image, by digest | its bytes as zero or more frames, each appending; zero frames is not held |
 * | `2` | `oci_prove`: the proof the client holds the image | one frame of 32 bytes; the empty finish is not held |
 *
 * In order: one `oci_manifest` for the digest the request names, and
 * one more for the platform manifest where that is an index; one
 * `oci_blob` for every blob the provider does not hold; then exactly
 * one `oci_prove`, always, last.
 */
export type Frame =
  | { readonly oci_manifest: manifest.Request }
  | { readonly oci_blob: blob.Request }
  | { readonly oci_prove: prove.Request };

const OCI_MANIFEST = 0;
const OCI_BLOB = 1;
const OCI_PROVE = 2;

/** An ask out of its payload; a tag that names none is the `unexpected_tag` error. */
export function decode(bytes: Uint8Array): Frame {
  const [tag, rest] = splitTag(bytes);
  switch (tag) {
    case OCI_MANIFEST:
      return { oci_manifest: manifest.Request.parse(jsonOf(rest)) };
    case OCI_BLOB:
      return { oci_blob: blob.Request.parse(jsonOf(rest)) };
    case OCI_PROVE:
      return { oci_prove: prove.Request.parse(jsonOf(rest)) };
    default:
      throw new DecodeError("unexpected_tag", tag);
  }
}
