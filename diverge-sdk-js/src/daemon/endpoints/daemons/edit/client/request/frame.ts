// A daemons edit request.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Identity } from "../../../../../../shared/identity.js";
import { Relay } from "../../../../../../shared/relay.js";
import { Ip } from "../../../../providers/ip.js";

/** Ask the daemon to change a record, by its name: every member given replaces the record's; each route filter is absent to leave it, `null` to clear it, or a list to replace it. A change drops a connection held to it. */
export const Frame = z.object({
  /** The record, by its name. */
  name: z.string(),
  /** The description, in place of the record's. */
  description: z.string().optional(),
  /** The remote, in place of the record's. */
  daemon: Identity.optional(),
  /** Our identity, in place of the record's, at the remote's issuer. */
  identity: Identity.optional(),
  /** The providers filter: absent leaves it, `null` clears it, a list replaces it. */
  providers: z.array(Identity).nullable().optional(),
  /** The IPs filter: absent leaves it, `null` clears it, a list replaces it. */
  ips: z.array(Ip).nullable().optional(),
  /** The addresses filter: absent leaves it, `null` clears it, a list replaces it. */
  addresses: z.array(z.string()).nullable().optional(),
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
