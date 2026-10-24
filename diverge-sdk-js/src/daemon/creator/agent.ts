// An agent as a creator.

import { z } from "zod";

/**
 * An agent of the client's that made something: named once and for
 * all by its template and its index, and by its name as it was
 * called.
 */
export const Agent = z.object({
  /** The template the agent was made from, by id. */
  template: z.string(),
  /** The agent's number among all agents of the client's ever made from that template. */
  index: z.number().int().nonnegative(),
  /** The agent's name, as its create gave it, if it gave one. Absent when it had none. */
  name: z.string().optional(),
});
export type Agent = z.infer<typeof Agent>;
