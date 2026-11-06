// The agent beginning to work: the mark the log keeps.

import { z } from "zod";

/**
 * The agent began working: a loop runs in it, or an MCP exchange is in
 * flight for it. `type` is the string `active`, and nothing lies beside
 * it. Kept only between the run's `deployed` and its `undeployed`, and
 * only on a change.
 */
export const Active = z.object({
  /** Which kind this is. */
  type: z.literal("active"),
});
export type Active = z.infer<typeof Active>;
