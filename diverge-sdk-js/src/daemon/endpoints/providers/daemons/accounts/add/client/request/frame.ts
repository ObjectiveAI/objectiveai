// Adding an account to a daemon record.

import { z } from "zod";
import { json, tagged } from "../../../../../../../../wire/encode.js";
import { Relay } from "../../../../../../../../shared/relay.js";
import { Mode } from "../../../../../../../../daemon/endpoints/providers/outgoing/mode.js";

/**
 * Ask the daemon to add an account to a daemon record: a name of the
 * caller's choosing, unique among the record's accounts, and the mode
 * this daemon authenticates to the remote in as that account — the
 * same modes as an outgoing provider's, the credential one the remote
 * daemon was told to expect as an incoming credential of its own. A
 * name the record has an account under already is refused.
 */
export const Frame = z.object({
  /** The daemon's name, as its add gave it. */
  daemon: z.string(),
  /** The account's name on it. */
  name: z.string(),
  /** How to authenticate there as this account, by the mode's name. */
  mode: Mode,
  /** Served by another daemon, as one of the accounts this daemon holds on it. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 98;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
