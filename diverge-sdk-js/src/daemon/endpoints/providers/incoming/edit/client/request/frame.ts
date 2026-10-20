// Replacing an incoming credential: a patch, and a new key.

import { z } from "zod";
import { json, tagged } from "../../../../../../../wire/encode.js";
import { Relay } from "../../../../../../../shared/relay.js";
import { Patch } from "../../../../../../../daemon/patch/operation.js";

/**
 * Ask the daemon to change a credential of incoming providers, by
 * identity: a patch to its editable document, the credential without
 * its key. Whatever the patch says, the daemon mints a new key and
 * answers it once — how a key rotates, and how an address or an
 * identity changes.
 */
export const Frame = z.object({
  /** The identity the credential names now. */
  identity: z.string(),
  /** The patch, applied to the `Editable` document. Empty, the key still rotates. */
  patch: Patch,
  /** Served by another daemon, as one of the accounts this daemon holds on it. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 42;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
