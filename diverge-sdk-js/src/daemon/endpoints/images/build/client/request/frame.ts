// Building an image on a provider.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";
import { Description } from "../../../../../../shared/description.js";
import { Identity } from "../../../../../agent_log/identity.js";
import { Context } from "./context.js";

/**
 * Ask the daemon to build an image on a provider: the provider's
 * `images::build`, asked by the daemon as itself, so the image goes
 * into the daemon's Store there and every container the daemon runs
 * on that provider can name its digest. Judged by `build` over
 * images and, where a context is named, `mount` over that volume.
 */
export const Frame = z.object({
  /** The provider to build on, and whose Store of the daemon's the image goes into. */
  provider: Identity,
  /** The Containerfile, whole, as text: what the build runs. */
  containerfile: z.string(),
  /**
   * What the image is kept as in the Store, beside its digest: one
   * short line of plain text. Absent, none. Not written over an image
   * owned already.
   */
  description: Description.optional(),
  /** The directory the build reads from. Absent, none, and a Containerfile that copies from its context fails. */
  context: Context.optional(),
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 157;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
