// One vault key as a get answers it: with every value.

import { z } from "zod";
import { Creator } from "../../../../../../daemon/creator/creator.js";
import { Agent } from "../../../../../../daemon/key/agent.js";
import { Tool } from "../../../../../../daemon/key/tool.js";
import { Slot } from "./slot.js";

/**
 * One vault key as a get answers it: what a `Key` reports, with every
 * value beside — the one place a value is sent.
 */
export const Entry = z.object({
  /** The name. */
  key: z.string(),
  /** Its values, each with its number and who holds it. In slot order, and never empty. */
  slots: z.array(Slot),
  /** The agents binding it. Absent when empty. */
  agents: z.array(Agent).optional(),
  /** The standalone tools binding it. Absent when empty. */
  standalone_tools: z.array(Tool).optional(),
  /** Its tags, sorted bytewise. Absent when empty. */
  tags: z.array(z.string()).optional(),
  /** When the first insert made it: an RFC 3339 timestamp in UTC. */
  created: z.iso.datetime({ offset: true }),
  /** Who made it, the direct maker. */
  creator: Creator,
});
export type Entry = z.infer<typeof Entry>;
