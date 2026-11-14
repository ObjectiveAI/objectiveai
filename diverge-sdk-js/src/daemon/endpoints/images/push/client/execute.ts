// Performing the exchange: one request, the daemon's asks answered,
// and what the push says.

import { CHUNK_SIZE } from "../../../../../index.js";
import { decodeServerFrame } from "../../../../../wire/frame/server.js";
import type { Handle, Scope } from "../../../../../wire/client/index.js";
import { ExecuteStream } from "../../../../../daemon/client/stream.js";
import { open } from "../../../../../daemon/client/one_shot.js";
import * as manifest from "../../../../../shared/containers/oci/manifest/response/frame.js";
import type * as prove from "../../../../../shared/containers/oci/prove/request/request.js";
import type { Image } from "../../../../../shared/containers/request/image.js";
import * as channel_request from "../server/channel_request/index.js";
import * as response from "../server/response/index.js";
import * as request from "./request/index.js";

/**
 * What a client holds of the image it pushes: the three things a
 * provider asks for. Each answers `undefined` for what is not held,
 * which is sent as the empty finish; a store that throws is one that
 * does not hold it. A client that holds no image answers `undefined`
 * to everything, and a push then succeeds only where a registry the
 * provider uses serves the image.
 */
export interface OciStore {
  /** The manifest under `digest`, of the push's `image`: its media type and its bytes, verbatim. */
  manifest(image: Image, digest: string): Promise<manifest.Frame | undefined>;
  /** The blob under `digest`, of the push's `image`, as its pieces in order. Piece sizes are the store's; each is sent sliced to `CHUNK_SIZE`. */
  blob(image: Image, digest: string): Promise<AsyncIterable<Uint8Array> | undefined>;
  /** The proof that `image` is held: the 32 bytes of `SHA-256(nonce ‖ D)` over it, for the request's platform and nonce. */
  prove(image: Image, request: prove.Request): Promise<Uint8Array | undefined>;
}

/**
 * Send the request, answer what the daemon asks of the image from
 * `store`, and hand back the stream of the push's answers: zero or
 * more blobs pulled, then the one answer that ends it, then the
 * daemon's finish.
 *
 * 1. The serving starts before the first answer is read: a provider
 *    that takes the image from the client asks for its manifest, its
 *    blobs and the proof before it answers anything but progress. Every
 *    channel the daemon opens on the scope is answered from `store`,
 *    several at once, until the scope ends.
 * 2. Every ask is finished, answered or not: the empty finish is "not
 *    held".
 * 3. There is no cancel: a push is not stopped from the client's side.
 */
export async function execute(handle: Handle, frame: request.Frame, store: OciStore): Promise<ExecuteStream<response.Frame>> {
  const scope = await open(handle, frame, request.encode);
  void serve(handle, scope, frame.image, store);
  return new ExecuteStream(scope, response.decode, true);
}

/** Every channel request the daemon opens on `scope`, each answered on its own, until the scope's requests end. A frame that is not a channel request is passed over. */
async function serve(handle: Handle, scope: Scope, image: Image, store: OciStore): Promise<void> {
  for (;;) {
    const bytes = await scope.requestReceiver.next();
    if (bytes === undefined) {
      return;
    }
    let envelope;
    try {
      envelope = decodeServerFrame(bytes);
    } catch {
      continue;
    }
    if (envelope.kind === "channel_request") {
      void answer(handle, scope.scope, envelope.channel, envelope.payload, image, store);
    }
  }
}

/** One ask answered from `store`, and then finished, whatever came of it: rule 2 of `execute`. */
async function answer(handle: Handle, scope: number, channel: number, payload: Uint8Array, image: Image, store: OciStore): Promise<void> {
  try {
    const ask = channel_request.decode(payload);
    if ("oci_manifest" in ask) {
      const held = await store.manifest(image, ask.oci_manifest.digest);
      if (held !== undefined) {
        await handle.sendChannelResponse(scope, channel, manifest.encode(held));
      }
    } else if ("oci_blob" in ask) {
      const pieces = await store.blob(image, ask.oci_blob.digest);
      if (pieces !== undefined) {
        for await (const piece of pieces) {
          for (let at = 0; at < piece.length; at += CHUNK_SIZE) {
            await handle.sendChannelResponse(scope, channel, piece.subarray(at, at + CHUNK_SIZE));
          }
        }
      }
    } else {
      const proof = await store.prove(image, ask.oci_prove);
      if (proof !== undefined) {
        await handle.sendChannelResponse(scope, channel, proof);
      }
    }
  } catch {
    // An ask that will not decode, or a store that failed: not held.
  }
  try {
    await handle.sendChannelResponseFinish(scope, channel);
  } catch {
    // The scope ended first: nothing is left to finish.
  }
}
