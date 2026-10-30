// A provider, as a log names one.

import { z } from "zod";
import { Identity } from "./identity.js";

/** The provider a container runs on: who it is. */
export const Provider = z.object({
  /** Who the provider is. */
  identity: Identity,
});
export type Provider = z.infer<typeof Provider>;
