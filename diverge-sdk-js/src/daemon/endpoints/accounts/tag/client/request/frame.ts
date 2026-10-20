// Putting tags on an account.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";
import { Reference } from "../../../../../../daemon/endpoints/accounts/reference.js";

/**
 * Ask the daemon to put tags on an account. A tag the account held already is held still, and
 * is not a failure. A tag is a string of the caller's choosing,
 * compared and not read; what an account holds is a set of them,
 * which its list item reports.
 */
export const Frame = z.object({
  /** The account. */
  account: Reference,
  /** The tags to put tags on, one by one. Empty changes nothing and is not a failure. */
  tags: z.array(z.string()),
  /** Served by another daemon, as one of the accounts this daemon holds on it. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 50;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
