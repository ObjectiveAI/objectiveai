// A run's container up on a provider.

import { z } from "zod";
import { Provider } from "./provider.js";

/**
 * The run's container came up on the provider named: the first item of
 * every run, before anything the run did. The provider's members lie
 * beside the item's own.
 */
export const Deployed = z.object({
  ...Provider.shape,
});
export type Deployed = z.infer<typeof Deployed>;
