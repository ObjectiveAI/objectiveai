// The agent's container down: the mark the log keeps.

import { z } from "zod";
import * as lifecycle from "../lifecycle/index.js";

/**
 * The agent's container went down, on any ending. `type` is the string
 * `undeployed`; the provider's members, the `reason` of an unexpected
 * end and `interrupted` lie beside it.
 */
export const Undeployed = z.object({
  /** Which kind this is. */
  type: z.literal("undeployed"),
  ...lifecycle.Undeployed.shape,
});
export type Undeployed = z.infer<typeof Undeployed>;
