// Putting tags on an agent template.

import { z } from "zod";
import { json, tagged } from "../../../../../../../wire/encode.js";
import { Relay } from "../../../../../../../shared/relay.js";

/**
 * Ask the daemon to put tags on an agent template. A tag held already is held still, and
 * is not a failure. A tag is a string of the caller's choosing,
 * compared and not read; what is held is a set of them, which the
 * list item reports.
 */
export const Frame = z.object({
    /** The template, by its id: the hash of the template itself. */
  id: z.string(),
  /** The tags to put tags on, one by one. Empty changes nothing and is not a failure. */
  tags: z.array(z.string()),
  /** Served by another daemon, as one of the accounts this daemon holds on it. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 13;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
