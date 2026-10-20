// One agent, once and for all.

import { z } from "zod";

/**
 * One agent of the caller's, by its template and its index, with
 * its name beside when it has one.
 */
export const Agent = z.object({
  /** The template it was made from, by id. */
  template: z.string(),
  /** Its number among all agents of the caller's ever made from that template. */
  index: z.number().int().nonnegative(),
  /** Its name, as its create gave it, if it gave one. Absent when it has none. */
  name: z.string().optional(),
});
export type Agent = z.infer<typeof Agent>;
