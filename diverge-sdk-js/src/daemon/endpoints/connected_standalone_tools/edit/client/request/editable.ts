// The connected tool as a patch sees it.

import { z } from "zod";

/**
 * The members of a connected tool an edit may change, as one JSON
 * document — and what its result must be again: every member
 * present, and no member this type does not have. A connected tool's
 * document is its name alone: what it is made from is the other
 * daemon's.
 */
export const Editable = z.strictObject({
  /** The name, or `null` for none. */
  name: z.string().nullable(),
});
export type Editable = z.infer<typeof Editable>;
