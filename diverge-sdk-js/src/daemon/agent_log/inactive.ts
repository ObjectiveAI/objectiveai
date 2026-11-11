// The agent ceasing to work: the mark the log keeps.

import { z } from "zod";

/**
 * The agent ceased working. `type` is the string `inactive`, and nothing
 * lies beside it. Kept only after an `active` of the same run, and never
 * after the run's `undeployed`; the agent's idle clock runs from here.
 */
export const Inactive = z.object({
  /** Which kind this is. */
  type: z.literal("inactive"),
});
export type Inactive = z.infer<typeof Inactive>;
