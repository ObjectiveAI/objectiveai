// Asking an agent something.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";
import { Agent } from "../../../../../../daemon/reference/agent.js";
import { Message } from "./message.js";

/**
 * Ask the daemon to ask an agent something: the agent, what is asked
 * — the `type` and that kind's members, beside the frame's own: see
 * `Message` — and whether to watch. The agent's container is started
 * for it if it does not run. Unwatched, the one answer comes when
 * the request's fate is known — handed to the agent's program, or
 * refused — which may be long after the ask; watched, the answer is
 * the items of the agent's log that are the request's. Nothing times
 * it out, and nothing takes the request back.
 */
export const Frame = z.intersection(
  z.object({
    /** The agent: by name, or by template and index. */
    agent: Agent,
    /**
     * Whether to watch: `true` has the answer be the part of the
     * agent's log that is the request's, as the response states.
     * Absent or `false`, the answer is the request's fate alone.
     */
    watch: z.boolean().optional(),
    /** Served by another daemon: see `Relay`. Absent, served here. */
    relay: Relay.optional(),
  }),
  Message,
);
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 3;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
