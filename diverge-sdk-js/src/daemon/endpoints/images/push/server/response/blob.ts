// A blob pulled from the client.

import { z } from "zod";

/** One blob the provider pulled from the client, through the daemon, and verified: the push's progress. Never sent for a blob the provider held already or took from a registry. */
export const Blob = z.object({
  /** The blob's digest, `<algorithm>:<hex>`. */
  digest: z.string(),
  /** Its size in BYTES. */
  bytes: z.number().int().nonnegative(),
});
export type Blob = z.infer<typeof Blob>;
