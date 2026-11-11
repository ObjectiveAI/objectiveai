// Putting tags on a dependency tool.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";
import { Dependency } from "../../../../../../daemon/reference/dependency.js";

/**
 * Ask the daemon to put tags on a dependency tool. A tag held already is held still, and
 * is not a failure. A tag is a string of the caller's choosing,
 * compared and not read; what is held is a set of them, which the
 * list item reports.
 */
export const Frame = z.object({
    /** The tool: its template by id, and whose it is. */
  dependency: Dependency,
  /** The tags to put tags on, one by one. Empty changes nothing and is not a failure. */
  tags: z.array(z.string()),
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 92;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
