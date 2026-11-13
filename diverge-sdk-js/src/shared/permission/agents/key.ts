// One agent, as a grant lists it.

import { z } from "zod";

/**
 * One agent a grant reaches, by what names it once and for all on the
 * daemon that holds it: the template it was made from and its number
 * among every agent that daemon ever made from that template. The
 * pair is given to nothing else ever, so a listed agent that is
 * deleted is reached by nothing after. No name is carried, and no
 * other member decodes.
 */
export const Key = z.strictObject({
  /** The template the agent was made from, by id. */
  template: z.string(),
  /** The agent's number among all agents the daemon ever made from that template. */
  index: z.number().int().nonnegative(),
});
export type Key = z.infer<typeof Key>;
