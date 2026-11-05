// A daemons edit request.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Identity } from "../../../../../../shared/identity.js";
import { Relay } from "../../../../../../shared/relay.js";

/** Ask the daemon to change a record, by its name: every member given replaces the record's. A change drops a connection held to it. */
export const Frame = z.object({
  /** The record, by its name. */
  name: z.string(),
  /** The description, in place of the record's. */
  description: z.string().optional(),
  /** The remote, in place of the record's. */
  daemon: Identity.optional(),
  /** Our identity, in place of the record's, at the remote's issuer. */
  identity: Identity.optional(),
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;


/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 131;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
