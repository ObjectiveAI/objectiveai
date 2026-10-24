// Changing a daemon's links: a patch.

import { z } from "zod";
import { json, tagged } from "../../../../../../../wire/encode.js";
import { Relay } from "../../../../../../../shared/relay.js";
import { Patch } from "../../../../../../../daemon/patch/operation.js";

/**
 * Ask the daemon to change a daemon record, by name: a patch to its
 * editable member, the links. The patch is applied to the record's
 * `Editable` document, whole or not at all; a link naming a provider
 * the caller has no record of is the edit's `no_provider`. The name
 * does not change, and the accounts are each their own edit's.
 */
export const Frame = z.object({
  /** The daemon's name. */
  name: z.string(),
  /** The patch, applied to the `Editable` document. Empty, nothing changes. */
  patch: Patch,
  /** Served by another daemon, as one of the accounts this daemon holds on it. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 86;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
