// Watching a dependency tool's container whole.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";
import { Dependency } from "../../../../../../daemon/reference/dependency.js";

/**
 * Ask the daemon for a running dependency tool's container's tree,
 * and every change to it for as long as the scope is open: the
 * container's own filesystem with each FUSE mount's subtree spliced
 * in at its mount point. There is no path to narrow it, since a
 * watch is of the whole container. A tool that does not run is
 * `not_found`; nothing is started.
 */
export const Frame = z.object({
  /** The tool: its template by id, and whose it is. */
  dependency: Dependency,
  /** Served by another daemon, as one of the accounts this daemon holds on it. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 94;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
