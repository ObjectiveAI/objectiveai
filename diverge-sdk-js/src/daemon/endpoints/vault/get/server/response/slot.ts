// One of a key's values, as a get answers it: with the value.

import { z } from "zod";
import { Creator } from "../../../../../../daemon/creator/creator.js";

/**
 * One of a key's values, as a get answers it: its number, who holds
 * it, and the value — the one place a value is sent. A run that takes
 * a `rotating` secret from the key takes exactly one of them, locked
 * to the run's record for its active period; a `fixed` secret reads
 * a random one and locks nothing.
 */
export const Slot = z.object({
  /** Which of the key's values this is: its number, `0` for the first appended. */
  slot: z.number().int().nonnegative(),
  /** Who holds it, when a run does. Absent, no one: it may be taken, replaced or deleted. */
  locked: Creator.optional(),
  /** The value, any JSON, as the last insert or write-back left it. */
  value: z.json(),
});
export type Slot = z.infer<typeof Slot>;
