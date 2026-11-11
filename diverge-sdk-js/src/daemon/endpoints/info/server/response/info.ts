// What the daemon says it is.

import { z } from "zod";

/**
 * What the daemon says it is: its version. A member this version does
 * not know is ignored, so a newer daemon's answer is read rather than
 * refused.
 */
export const Info = z.object({
  /** The daemon's version, `MAJOR.MINOR.PATCH`: the revision of the protocol it speaks. */
  version: z.string(),
});
export type Info = z.infer<typeof Info>;
