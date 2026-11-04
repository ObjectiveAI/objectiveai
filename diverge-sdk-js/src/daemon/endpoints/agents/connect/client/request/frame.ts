// Connecting to an agent, for MCP as it.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";
import { Agent } from "../../../../../../daemon/reference/agent.js";

/**
 * Ask the daemon to answer MCP exchanges on this scope as the agent's
 * own. Nothing is started by the connect: each exchange starts the
 * agent's container if it is not up, and the scope alone holds
 * nothing up.
 */
export const Frame = z.object({
  /** The agent: by name, or by template and index. */
  agent: Agent,
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 124;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
