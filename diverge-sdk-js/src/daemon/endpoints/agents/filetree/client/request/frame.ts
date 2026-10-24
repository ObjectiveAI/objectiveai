// Watching an agent's container whole.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";
import { Agent } from "../../../../../../daemon/reference/agent.js";

/**
 * Ask the daemon for an agent's container's tree, and every change
 * to it for as long as the scope is open: the container's own
 * filesystem with each FUSE mount's subtree spliced in at its mount
 * point. There is no path to narrow it, since a watch is of the
 * whole container.
 */
export const Frame = z.object({
  /** The agent: by name, or by template and index. */
  agent: Agent,
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 76;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
