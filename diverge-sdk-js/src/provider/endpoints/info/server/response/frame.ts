// What a server's response frame carries for an info request.

import { z } from "zod";
import { Capabilities } from "./capabilities.js";

/**
 * What the provider says it is: the revision of the specification it
 * implements — `MAJOR.MINOR.PATCH`, the crate's own — and the
 * capabilities it holds out. On the provider's wire the struct as
 * JSON and nothing in front of it, there being nothing to
 * discriminate; a daemon's record embeds it as it came. A member this
 * version does not know is ignored.
 */
export const Frame = z.looseObject({
  /** The revision: `MAJOR.MINOR.PATCH`. */
  version: z.string(),
  /** What the provider does. Absent, nothing beyond what every provider serves. */
  capabilities: Capabilities.optional(),
});
export type Frame = z.infer<typeof Frame>;
