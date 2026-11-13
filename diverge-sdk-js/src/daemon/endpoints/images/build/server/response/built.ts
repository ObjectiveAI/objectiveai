// The image, built.

import { z } from "zod";

/** The image built into the daemon's Store on the provider, as the provider gave it. */
export const Built = z.object({
  /** The manifest digest of the image as the provider holds it, `<algorithm>:<hex>`: what a template's image names. */
  digest: z.string(),
  /** Its size in BYTES, as the provider counts it against the Store's capacity. */
  bytes: z.number().int().nonnegative(),
});
export type Built = z.infer<typeof Built>;
