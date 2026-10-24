// Deleting a volume.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";
import { Volume } from "../../../../../../daemon/reference/volume.js";

/**
 * Ask the daemon to delete a volume. A volume in use — named in a
 * container's mounts, or with a download, an upload or a transfer on
 * it — is not deleted, and the daemon says so with a variant of its
 * own. Gone is gone: the volume no longer exists.
 */
export const Frame = z.object({
  /** The volume: its provider and its name. */
  volume: Volume,
  /** Served by another daemon, as one of the accounts this daemon holds on it. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 68;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
