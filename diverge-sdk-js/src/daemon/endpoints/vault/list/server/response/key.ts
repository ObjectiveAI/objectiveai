// One vault key, as a list reports it.

import { z } from "zod";
import { Creator } from "../../../../../../daemon/creator/creator.js";
import { Agent } from "../../../../../../daemon/key/agent.js";
import { Tool } from "../../../../../../daemon/key/tool.js";
import type { Keyed } from "../../../../../../shared/listing/keyed.js";
import { SlotState } from "./slot_state.js";

/**
 * One vault key: its name, its slots with who holds each, the agents
 * and tools binding it, its tags, when it was made and by whom — and
 * never a value, which a `get` alone answers. A list sends them
 * oldest created first.
 */
export const Key = z.object({
  /** The name. */
  key: z.string(),
  /** Its values, each by number with who holds it and never what it is. In slot order, and never empty. */
  slots: z.array(SlotState),
  /** The agents binding it in their `secrets`. Absent when empty. */
  agents: z.array(Agent).optional(),
  /** The standalone tools binding it, likewise. Absent when empty. */
  standalone_tools: z.array(Tool).optional(),
  /** Its tags, sorted bytewise. Absent when empty. */
  tags: z.array(z.string()).optional(),
  /** When the first insert made it: an RFC 3339 timestamp in UTC. */
  created: z.iso.datetime({ offset: true }),
  /** Who made it, the direct maker. */
  creator: Creator,
});
export type Key = z.infer<typeof Key>;

/** Keyed by the key's name. */
export const key: Keyed<Key> = (entry) => entry.key;
