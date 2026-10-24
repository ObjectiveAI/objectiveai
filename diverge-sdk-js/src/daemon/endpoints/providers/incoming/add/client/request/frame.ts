// Adding a credential of incoming providers.

import { z } from "zod";
import { json, tagged } from "../../../../../../../wire/encode.js";
import { Relay } from "../../../../../../../shared/relay.js";
import { Credential } from "../../../../../../../daemon/endpoints/providers/incoming/credential.js";

/**
 * Ask the daemon to add a credential of incoming providers: the
 * identity a provider presenting it will have, and the address it is
 * accepted from if one, flat among the members. The key is not here:
 * the daemon mints it and answers it, once. A credential for an
 * identity there is one for already is refused.
 */
export const Frame = z.object({
  ...Credential.shape,
  /** Served by another daemon, as one of the accounts this daemon holds on it. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 38;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
