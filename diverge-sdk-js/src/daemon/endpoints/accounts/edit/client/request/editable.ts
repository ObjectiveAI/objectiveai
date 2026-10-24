// The document an account's patch is applied to.

import { z } from "zod";

/**
 * The members of an account an edit may change, as one JSON
 * document: what a patch is applied to, and what its result must be
 * again — every member present, a member the patch takes away read
 * as absent or empty, and no member this type does not have. The
 * identity is the account, and is not here: another identity is
 * another account.
 */
export const Editable = z.strictObject({
  /** What the account is for, in words. `null`, none. */
  description: z.string().nullable(),
  /** The roles it holds, by name, each one the daemon has and the caller holds the `grant` grant over. Empty, the account may do nothing. */
  roles: z.array(z.string()),
});
export type Editable = z.infer<typeof Editable>;
