// Detaching a connected tool from every agent of a template.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";
import { ConnectedStandaloneTool } from "../../../../../../daemon/reference/connected_standalone_tool.js";

/**
 * Ask the daemon to detach a connected tool from every agent made from an agent template,
 * now and later. An agent attached directly as well keeps the tool.
 */
export const Frame = z.object({
  /** The connected tool: by its name, or by the daemon, the tool there and its index. */
  tool: ConnectedStandaloneTool,
  /** The agent template, by id. */
  template: z.string(),
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 156;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
