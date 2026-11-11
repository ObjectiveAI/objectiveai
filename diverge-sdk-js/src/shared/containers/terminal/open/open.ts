// What a terminal starts at.

import { z } from "zod";

/**
 * Start a terminal this many characters wide and this many lines
 * tall: what programs lay their output out to, until a `Resize`.
 */
export const Open = z.object({
  /** Characters across a line. */
  cols: z.number().int().min(0).max(65535),
  /** Lines down the screen. */
  rows: z.number().int().min(0).max(65535),
});
export type Open = z.infer<typeof Open>;
