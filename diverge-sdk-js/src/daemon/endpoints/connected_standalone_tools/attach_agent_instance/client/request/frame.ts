// Attaching a connected tool to one agent.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";
import { ConnectedStandaloneTool } from "../../../../../../daemon/reference/connected_standalone_tool.js";
import { Agent } from "../../../../../../daemon/reference/agent.js";

/**
 * Ask the daemon to attach a connected tool to one agent, directly. Lands live on an active agent, which is told its tools changed; the other daemon's connect is opened at the agent's next use. Attaching one attached already changes nothing.
 */
export const Frame = z.object({
  /** The connected tool: by its name, or by the daemon, the tool there and its index. */
  tool: ConnectedStandaloneTool,
  /** The agent: by name, or by template and index. */
  agent: Agent,
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 153;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
