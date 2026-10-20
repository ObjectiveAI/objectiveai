// Deleting an agent.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";
import { Agent } from "../../../../../../daemon/reference/agent.js";

/**
 * Ask the daemon to delete an agent: its record, its log and its
 * container, whether or not it runs — unless a loop is running in
 * it, which is `active`, and the agent is left alone.
 */
export const Frame = z.object({
  /** The agent: by name, or by template and index. */
  agent: Agent,
  /** Served by another daemon, as one of the accounts this daemon holds on it. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 2;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
