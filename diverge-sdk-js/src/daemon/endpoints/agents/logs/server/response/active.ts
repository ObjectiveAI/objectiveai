// A run began: the mark the log keeps.

import { z } from "zod";
import { Provider } from "./provider.js";

/**
 * The agent became active: its container came up on the provider named,
 * and the agent is running there. `type` is the string `active`, and
 * the provider's members lie beside it.
 *
 * Kept when the daemon has the agent running on a provider. Every chunk
 * that follows, until the `Inactive` that matches, was produced there.
 * A run that never came up keeps an `Error` and no `active`.
 */
export const Active = z.object({
  /** Which kind this is. */
  type: z.literal("active"),
  ...Provider.shape,
});
export type Active = z.infer<typeof Active>;
