// Naming a volume: its provider, and its name there.

import { z } from "zod";
import { Identity } from "../endpoints/agents/logs/server/response/identity.js";

/**
 * One volume: the provider that holds it, by the daemon's `Identity`
 * for it, and the name that provider lists it under. A volume is a
 * provider's own and has no name anywhere else, so the pair is the
 * whole of how one is named. One JSON object,
 * `{"provider":…,"name":…}`.
 */
export const Volume = z.object({
  /** The provider, by its identity: `{"issuer":…,"subject":…}`. */
  provider: Identity,
  /** The name the provider lists the volume under. */
  name: z.string(),
});
export type Volume = z.infer<typeof Volume>;
