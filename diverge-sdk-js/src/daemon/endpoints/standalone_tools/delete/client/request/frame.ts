// Deleting a tool.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";
import { Tool } from "../../../../../../daemon/reference/tool.js";

/**
 * Ask the daemon to delete a tool: its record and its container,
 * whether or not it runs — unless it is attached anywhere, which is
 * `attached`, and the tool stays. A deleted name is free again.
 */
export const Frame = z.object({
  /** The tool: by name, by template and index, or another daemon's by the daemon, the account and its reference there. */
  tool: Tool,
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 21;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
