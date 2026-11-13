// The image, owned already.

import { z } from "zod";

/**
 * The build produced a digest the daemon already owns in its Store
 * on the provider: nothing was added, the image there is the one
 * asked for, and its description is what it was.
 */
export const Exists = z.object({
  /** The manifest digest the build produced, `<algorithm>:<hex>`. */
  digest: z.string(),
});
export type Exists = z.infer<typeof Exists>;
