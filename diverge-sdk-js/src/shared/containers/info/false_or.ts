// Not at all, or as the given schema says.

import { z } from "zod";

/**
 * Something an image's program does not do, or does as `is` says: the
 * boolean `false`, or `is` as it is, an object. `true` is not one of
 * them, and neither is any other value.
 */
export function falseOr<T extends z.ZodType>(is: T) {
  return z.union([z.literal(false), is]);
}
