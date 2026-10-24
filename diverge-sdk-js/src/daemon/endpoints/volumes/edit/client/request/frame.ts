// Changing a volume: a patch to how many bytes it reserves and its mode.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";
import { Volume } from "../../../../../../daemon/reference/volume.js";
import { Patch } from "../../../../../../daemon/patch/operation.js";

/**
 * Ask the daemon to change a volume: a patch to its editable members
 * — how many bytes it reserves, its mode. The patch is applied to the
 * volume's `Editable` document, as the provider lists it; a result
 * that differs is asked of the provider as one edit, and one that
 * does not is `edited` with nothing asked. A volume's name does not
 * change.
 */
export const Frame = z.object({
  /** The volume: its provider and its name. */
  volume: Volume,
  /** The patch, applied to the `Editable` document. Empty, nothing changes. */
  patch: Patch,
  /** Served by another daemon, as one of the accounts this daemon holds on it. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 69;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
