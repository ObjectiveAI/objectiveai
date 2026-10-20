// Adding a daemon: another daemon the caller holds accounts on.

import { z } from "zod";
import { json, tagged } from "../../../../../../../wire/encode.js";
import { Relay } from "../../../../../../../shared/relay.js";
import { Link } from "../../../../../../../daemon/endpoints/providers/daemons/link.js";

/**
 * Ask the daemon to add a daemon: another daemon the caller holds
 * accounts on, under a name of the caller's choosing, with the links
 * it is reached through. Its accounts are added one by one
 * afterwards. A name the caller has given a daemon already is
 * refused; so is a link naming a provider the caller has no record
 * of.
 */
export const Frame = z.object({
  /** The name, of the caller's choosing: the daemon's identity here from then on. */
  name: z.string(),
  /** The providers the daemon is reached through, each with the identity it is known by there. Absent, none until an edit gives it one. */
  links: z.array(Link).optional(),
  /** Served by another daemon, as one of the accounts this daemon holds on it. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 82;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
