// Sending an agent a message.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";
import { Agent } from "../../../../../../daemon/reference/agent.js";
import { ContentBlockSchema } from "@modelcontextprotocol/core";

/**
 * Ask the daemon to hand an agent a message: content blocks, MCP's
 * own, relayed to the container verbatim and not read. The agent's
 * container is started for it if it does not run. The answer comes
 * when the message's fate is known — taken by the loop, or
 * cancelled first — which may be long after the ask; nothing times
 * it out. An image that cannot take a block refuses the whole
 * message, and the error carries its words.
 */
export const Frame = z.object({
  /** The agent: by name, or by template and index. */
  agent: Agent,
  /** The message: content blocks, as MCP writes them. */
  content: z.array(ContentBlockSchema),
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 3;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
