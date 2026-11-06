// The agent's container up: the mark the log keeps.

import { z } from "zod";
import * as lifecycle from "../lifecycle/index.js";

/**
 * The agent's container came up on the provider named. `type` is the
 * string `deployed`, and the provider's members lie beside it. A run
 * that never came up keeps an `Error` and no `deployed`.
 */
export const Deployed = z.object({
  /** Which kind this is. */
  type: z.literal("deployed"),
  ...lifecycle.Deployed.shape,
});
export type Deployed = z.infer<typeof Deployed>;
