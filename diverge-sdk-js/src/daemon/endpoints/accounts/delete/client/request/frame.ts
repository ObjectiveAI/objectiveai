// Deleting an account.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";
import { Reference } from "../../../../../../daemon/endpoints/accounts/reference.js";

/**
 * Ask the daemon to delete an account. An account a container runs
 * under, or a client is connected as, is not deleted, and the daemon
 * says so with a variant of its own: delete the containers, or wait
 * for the client to go, and ask again.
 */
export const Frame = z.object({
  /** The account. */
  account: Reference,
  /** Served by another daemon, as one of the accounts this daemon holds on it. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 48;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
