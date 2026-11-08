// The channels a caller opens on an oci scope.

import { tagged } from "../../../../../../wire/encode.js";
import * as blob from "../../../../../../shared/containers/oci/blob/request/request.js";
import * as manifest from "../../../../../../shared/containers/oci/manifest/request/request.js";
import * as prove from "../../../../../../shared/containers/oci/prove/request/request.js";

/**
 * What a caller asks for on an oci scope answered held: three of the
 * asks of `shared.containers.oci`, each over the scope's image, and
 * the cancel that ends the scope.
 *
 * | tag | asks for |
 * |-----|----------|
 * | `0` | `oci_manifest`: a manifest by digest; one frame, or the empty finish |
 * | `1` | `oci_blob`: a blob by digest; pieces, each appending, then the finish |
 * | `2` | `oci_prove`: the proof over the image; one frame of 32 bytes, or the empty finish |
 * | `3` | `cancel`: carries nothing, and nothing answers on its channel; the scope's finish does |
 */
export type Frame =
  | { readonly oci_manifest: manifest.Request }
  | { readonly oci_blob: blob.Request }
  | { readonly oci_prove: prove.Request }
  | "cancel";

const OCI_MANIFEST = 0;
const OCI_BLOB = 1;
const OCI_PROVE = 2;
const CANCEL = 3;

/** One byte of tag, then the ask's own JSON; the cancel is the tag alone. */
export function encode(frame: Frame): Uint8Array {
  if (frame === "cancel") {
    return Uint8Array.of(CANCEL);
  }
  if ("oci_manifest" in frame) {
    return tagged(OCI_MANIFEST, manifest.encode(frame.oci_manifest));
  }
  if ("oci_blob" in frame) {
    return tagged(OCI_BLOB, blob.encode(frame.oci_blob));
  }
  return tagged(OCI_PROVE, prove.encode(frame.oci_prove));
}
