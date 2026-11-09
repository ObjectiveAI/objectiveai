// Editing a connected tool's name.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";
import { ConnectedStandaloneTool } from "../../../../../../daemon/reference/connected_standalone_tool.js";
import { Patch } from "../../../../../../daemon/patch/operation.js";

/**
 * Ask the daemon to change a connected tool: a JSON Patch over its
 * `Editable` — its name, and nothing else — applied whole or not at
 * all. Empty changes nothing.
 */
export const Frame = z.object({
  /** The connected tool: by its name, or by the daemon, the tool there and its index. */
  tool: ConnectedStandaloneTool,
  /** The operations, in order: see `daemon.patch`. Empty changes nothing. */
  patch: Patch,
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 149;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
