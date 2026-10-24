// Creating a role: a name, what it is for, and its grants.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";
import { Grant } from "../../../../../../daemon/grant/grant.js";

/**
 * Ask the daemon to create a role under a name. A name that is a
 * role's already is refused. The grants are the caller's to write;
 * the daemon reads them no further than decoding them.
 */
export const Frame = z.object({
  /** The name, unique among roles; compared and not read. */
  name: z.string(),
  /** What the role is for, in words of the caller's. Absent, none. */
  description: z.string().optional(),
  /** The grants. Empty, a role that allows nothing, which is not a failure. */
  grants: z.array(Grant).optional(),
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 52;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
