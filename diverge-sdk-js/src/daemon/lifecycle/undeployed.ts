// A run's container down.

import { z } from "zod";
import { Provider } from "./provider.js";
import { Reason } from "./reason.js";

/**
 * The run's container went down: the last item of every run, naming the
 * provider its `deployed` named. `reason` is present only when the end
 * was unexpected; `interrupted` is whether the run was working, and
 * when it was, an `inactive` precedes this item.
 */
export const Undeployed = z.object({
  ...Provider.shape,
  /** Why the run ended unexpectedly; absent when it was asked to. */
  reason: Reason.optional(),
  /** Whether the run was working when it ended. */
  interrupted: z.boolean(),
});
export type Undeployed = z.infer<typeof Undeployed>;
