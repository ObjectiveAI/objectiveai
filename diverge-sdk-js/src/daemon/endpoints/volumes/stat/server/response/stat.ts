// What a walk of a volume finds.

import { z } from "zod";

/**
 * The two things a listing does not say about a volume, as of the
 * moment of the walk: how much of it is used, and the hash of its
 * content.
 */
export const Stat = z.object({
  /** How much of the volume is in use, in bytes, as of the walk. */
  bytes_used: z.number().int().nonnegative(),
  /** The hash of the volume's content, as of the walk: Go's `dirhash` of its root, the `h1:` string. */
  dirhash: z.string(),
});
export type Stat = z.infer<typeof Stat>;
