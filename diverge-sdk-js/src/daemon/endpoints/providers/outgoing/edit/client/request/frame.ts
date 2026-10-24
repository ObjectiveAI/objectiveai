// Changing how an outgoing provider is dialled: a patch to its mode.

import { z } from "zod";
import { json, tagged } from "../../../../../../../wire/encode.js";
import { Relay } from "../../../../../../../shared/relay.js";
import { Patch } from "../../../../../../../daemon/patch/operation.js";

/**
 * Ask the daemon to change an outgoing provider, by address: a patch
 * to its editable member, the mode. The patch is applied to the
 * provider's `Editable` document — whose mode is `null`, since the
 * daemon answers a credential to nobody — so the one patch that
 * changes anything gives the mode whole, which is how a credential
 * rotates. The next dial presents the new credential.
 */
export const Frame = z.object({
  /** The address. */
  address: z.string(),
  /** The patch, applied to the `Editable` document. Empty, nothing changes. */
  patch: Patch,
  /** Served by another daemon, as one of the accounts this daemon holds on it. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 35;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
