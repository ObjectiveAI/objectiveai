// The image, owned already.

import { z } from "zod";

/** The daemon already owns the digest in its Store on the provider: nothing was pushed, and its description is what it was. */
export const Exists = z.object({
  /** The manifest digest, `<algorithm>:<hex>`, as the request named it. */
  digest: z.string(),
});
export type Exists = z.infer<typeof Exists>;
