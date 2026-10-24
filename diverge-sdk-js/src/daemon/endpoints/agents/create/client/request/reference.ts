// One place an image's bytes may be fetched from.

import { z } from "zod";

/**
 * One place the bytes an image's digest names may be fetched from:
 * a registry, and the repository path there.
 */
export const Reference = z.object({
  /** The registry's host, as a reference names it before its first `/`. No scheme and no path. */
  registry: z.string(),
  /** The repository path at that registry. No host and no tag. */
  name: z.string(),
});
export type Reference = z.infer<typeof Reference>;
