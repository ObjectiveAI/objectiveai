// Rotating an account on a daemon record: a patch to its mode.

import { z } from "zod";
import { json, tagged } from "../../../../../../../../wire/encode.js";
import { Relay } from "../../../../../../../../shared/relay.js";
import { Patch } from "../../../../../../../../daemon/patch/operation.js";

/**
 * Ask the daemon to change an account on a daemon record: a patch to
 * its editable member, the mode, presented as `null` and given whole
 * by the patch — how a credential rotates. A connection this daemon
 * holds as the account now is not dropped; the next one presents the
 * new credential.
 */
export const Frame = z.object({
  /** The daemon's name, as its add gave it. */
  daemon: z.string(),
  /** The account's name on it. */
  name: z.string(),
  /** The patch, applied to the `Editable` document. Empty, nothing changes. */
  patch: Patch,
  /** Served by another daemon, as one of the accounts this daemon holds on it. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 99;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
