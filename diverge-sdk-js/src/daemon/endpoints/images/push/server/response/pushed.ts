// The image, pushed.

import { z } from "zod";
import { Source } from "../../../../../../shared/containers/response/source.js";

/** The image pushed into the daemon's Store on the provider, as the provider gave it. */
export const Pushed = z.object({
  /** The manifest digest of the image as the provider holds it, `<algorithm>:<hex>`: what a template's image names. */
  digest: z.string(),
  /** Its size in BYTES, as the provider counts it against the Store's capacity. */
  bytes: z.number().int().nonnegative(),
  /** Where the provider took it from: a registry it uses, or the client, who proved it holds it. Never `store`. */
  source: Source,
});
export type Pushed = z.infer<typeof Pushed>;
