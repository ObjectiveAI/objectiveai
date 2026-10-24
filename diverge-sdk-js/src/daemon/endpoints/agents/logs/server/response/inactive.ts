// A run ended: the mark the log keeps.

import { z } from "zod";
import { Provider } from "./provider.js";

/**
 * The agent became inactive: it no longer runs on the provider named.
 * `type` is the string `inactive`, and the provider's members lie
 * beside it.
 *
 * Kept when the agent ceases to run on a provider, on any ending — the
 * run's own finish, the daemon's stop, the provider's loss. The
 * provider is the one the last `Active` named, so the two bracket every
 * chunk produced there; nothing between an `inactive` and the next
 * `active` was said by the agent.
 */
export const Inactive = z.object({
  /** Which kind this is. */
  type: z.literal("inactive"),
  ...Provider.shape,
});
export type Inactive = z.infer<typeof Inactive>;
