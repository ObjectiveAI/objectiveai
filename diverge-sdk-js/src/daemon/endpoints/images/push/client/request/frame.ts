// Asking the daemon to push an image into its Store on a provider.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Identity } from "../../../../../agent_log/identity.js";
import { Image } from "../../../../../../shared/containers/request/image.js";
import { Description } from "../../../../../../shared/description.js";
import { Relay } from "../../../../../../shared/relay.js";

/**
 * Ask the daemon to push an image into its Store on a provider: the
 * provider's `images.push`, asked by the daemon as itself.
 */
export const Frame = z.object({
  /** The provider to push to, and whose Store of the daemon's the image goes into. */
  provider: Identity,
  /**
   * The image: its digest, which is the image, and the references
   * under which its bytes may be fetched. A reference naming a
   * registry the provider uses lets the provider fetch it there; with
   * none that does, the client is asked for it.
   */
  image: Image,
  /** What the image is kept as in the Store, beside its digest: one short line. Absent, none. Not written over an image owned already. */
  description: Description.optional(),
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 160;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
