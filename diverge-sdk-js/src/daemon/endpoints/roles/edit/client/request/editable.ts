// The document a role's patch is applied to.

import { z } from "zod";
import { Grant } from "../../../../../../daemon/grant/grant.js";

/**
 * The members of a role an edit may change, as one JSON document:
 * what a patch is applied to, and what its result must be again —
 * every member present, and no member this type does not have. The
 * name is not among them: a role is reached by its name alone, so a
 * new name is a new role.
 */
export const Editable = z.strictObject({
  /** What the role is for, in words. `null`, none. */
  description: z.string().nullable(),
  /** The grants, in order. Empty, the role allows nothing. */
  grants: z.array(Grant),
});
export type Editable = z.infer<typeof Editable>;
