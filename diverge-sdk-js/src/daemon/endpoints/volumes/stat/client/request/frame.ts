// Walking a volume: how much of it is used, and the hash of its content.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";
import { Volume } from "../../../../../../daemon/reference/volume.js";

/**
 * Ask the daemon to walk a volume: the provider's `volumes.stat`,
 * relayed. The provider walks the volume once, at rest, and answers
 * the two numbers a listing leaves out because each costs the walk.
 */
export const Frame = z.object({
  /** The volume: its provider and its name. */
  volume: Volume,
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 72;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
