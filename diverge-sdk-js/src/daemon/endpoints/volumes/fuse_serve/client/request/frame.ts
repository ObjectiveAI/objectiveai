// Asking the daemon to serve a volume over FUSE.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Mode } from "../../../../../../provider/endpoints/volumes/mode.js";
import { Relay } from "../../../../../../shared/relay.js";
import { Volume } from "../../../../../../daemon/reference/volume.js";

/**
 * Ask the daemon to serve a volume over FUSE: the provider's
 * `volumes.fuse_serve` request, the volume named as the daemon names
 * one.
 */
export const Frame = z.object({
  /** The volume: the provider's identity and the name. */
  volume: Volume,
  /** The mode the volume is meant to be in. A volume in another mode is refused, with the mode it is in. */
  mode: Mode,
  /**
   * The most bytes the serve's own layer may hold, for an ephemeral
   * volume, whose changes land in that layer and go with the serve.
   * Absent, none: what a persistent or a read-only volume states.
   */
  overlay_disk: z.number().int().nonnegative().optional(),
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 158;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
