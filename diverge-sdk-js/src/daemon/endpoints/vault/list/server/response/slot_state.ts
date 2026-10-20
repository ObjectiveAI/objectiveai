// One of a key's values as a list reports it: its number and who holds it.

import { z } from "zod";
import { Creator } from "../../../../../../daemon/creator/creator.js";

/**
 * One of a key's values as a list reports it: its number and who
 * holds it, and never the value, which a `get` alone answers.
 */
export const SlotState = z.object({
  /** Which of the key's values this is: its number, `0` for the first appended. */
  slot: z.number().int().nonnegative(),
  /** Who holds it, when a run does: the record whose container took it as a `rotating` secret. Absent, no one. */
  locked: Creator.optional(),
});
export type SlotState = z.infer<typeof SlotState>;
