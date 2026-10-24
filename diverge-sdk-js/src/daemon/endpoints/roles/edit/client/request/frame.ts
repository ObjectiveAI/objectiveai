// Changing a role: a patch to its description and its grants.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";
import { Patch } from "../../../../../../daemon/patch/operation.js";

/**
 * Ask the daemon to change a role: a patch to its editable members —
 * its description, its grants. The patch is applied to the role's
 * `Editable` document, whole or not at all, and the result is the
 * role from then on; every account holding it is judged by the new
 * grants from its next request on. A role's name does not change.
 */
export const Frame = z.object({
  /** The role to change, by name. */
  name: z.string(),
  /** The patch, applied to the `Editable` document. Empty, nothing changes. */
  patch: Patch,
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 56;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
