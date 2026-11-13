// Reading an image's DIVERGE.md.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Identity } from "../../../../../../shared/identity.js";
import { Relay } from "../../../../../../shared/relay.js";
import { Image } from "../../../../../../shared/containers/request/image.js";

/**
 * Ask the daemon for an image's `/DIVERGE.md`. The digest is the
 * image; the references are where a provider may fetch it from, when
 * nothing of it runs.
 */
export const Frame = z.object({
  /** The image: its digest, and the references its bytes may be fetched under. */
  image: Image,
  /**
   * The remote daemons the daemon does not ask, by the identities
   * they answer as. A daemon asking another sends every identity it
   * was itself told to ignore, and its own. Absent, none.
   */
  ignore: z.array(Identity).optional(),
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 110;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
