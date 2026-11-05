// One identity, as a list answers it.

import { z } from "zod";

/** One identity signed in on the daemon's host: its issuer, its subject, and the tags the daemon keeps on it. No token is ever answered. */
export const Identity = z.object({
  /** The issuer, exactly as its tokens' `iss` names it. */
  issuer: z.string(),
  /** The subject: its tokens' `sub`. */
  subject: z.string(),
  /** The tags on it, sorted; none where it has none. */
  tags: z.array(z.string()),
});
export type Identity = z.infer<typeof Identity>;
