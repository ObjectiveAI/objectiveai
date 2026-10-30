// One volume, as a provider lists it and the daemon knows it.

import { z } from "zod";
import { Identity } from "../../../../../agent_log/identity.js";
import { Agent } from "../../../../../../daemon/key/agent.js";
import { Tool } from "../../../../../../daemon/key/tool.js";
import * as reference from "../../../../../../daemon/reference/volume.js";
import { Mode } from "../../../../../../provider/endpoints/volumes/mode.js";
import type { Keyed } from "../../../../../../shared/listing/keyed.js";

/**
 * One volume: the provider that holds it, and what that provider's
 * listing says of it — its name, how many bytes it reserves, when it
 * came into being, its mode — with the agents and the tools of the
 * daemon's that name it in their mounts, and its tags, which are the
 * daemon's. No creator: a volume is the provider's. How much of it is
 * used, and the hash of its content, cost the provider a walk, and
 * are a `stat`'s.
 */
export const Volume = z.object({
  /** The provider that holds it. */
  provider: Identity,
  /** The name the provider lists it under: the whole of how it is named, with the provider. */
  name: z.string(),
  /** How many bytes it reserves, as the provider lists it. */
  bytes: z.number().int().nonnegative(),
  /** The mode it is in. */
  mode: Mode,
  /** When it came into being, as the provider says: an RFC 3339 timestamp in UTC. */
  created: z.iso.datetime({ offset: true }),
  /** The agents that name it in their mounts, running or not. Absent when empty. */
  agents: z.array(Agent).optional(),
  /** The tools that name it in their mounts. Absent when empty. */
  tools: z.array(Tool).optional(),
  /** The tags on it: the daemon's, not the provider's. Absent when empty. */
  tags: z.array(z.string()).optional(),
});
export type Volume = z.infer<typeof Volume>;

/** Keyed by the provider and the name, as a volume is referred to. */
export const key: Keyed<Volume> = (volume) => ({ provider: volume.provider, name: volume.name }) satisfies reference.Volume;
