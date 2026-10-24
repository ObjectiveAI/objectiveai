// Taking out an incoming credential.

import { z } from "zod";
import { json, tagged } from "../../../../../../../wire/encode.js";
import { Relay } from "../../../../../../../shared/relay.js";

/**
 * Ask the daemon to take out a credential of incoming providers, by
 * identity. A credential a provider is connected through is not taken
 * out, and the daemon says so with a variant of its own.
 */
export const Frame = z.object({
  /** The identity the credential names. */
  identity: z.string(),
  /** Served by another daemon, as one of the accounts this daemon holds on it. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 41;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
