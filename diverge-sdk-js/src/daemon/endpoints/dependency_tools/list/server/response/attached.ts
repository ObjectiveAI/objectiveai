// One agent that depends on the tool.

import { z } from "zod";
import { Agent } from "../../../../../../daemon/key/agent.js";

/**
 * One agent whose last registration declared the tool's template, and
 * when it last did. An instance tool has at most one, its owner; a
 * per-template tool the agents of its agent template that declare it; a
 * global tool every agent that does.
 */
export const Attached = z.object({
  /** The agent, by its `key`: its template and its index, with its name beside when it has one. */
  agent: Agent,
  /** When the agent's registration last declared the template. On the wire an RFC 3339 timestamp in UTC. Every registration that declares it moves this forward; one that leaves it out takes the agent off the tool. */
  declared: z.iso.datetime({ offset: true }),
});
export type Attached = z.infer<typeof Attached>;
