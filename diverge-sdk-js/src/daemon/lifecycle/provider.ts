// A provider, as a run's life names one.

import { z } from "zod";
import { Identity } from "../agent_log/identity.js";

/** The provider a run is deployed on: who it is. */
export const Provider = z.object({
  /** Who the provider is. */
  identity: Identity,
});
export type Provider = z.infer<typeof Provider>;
