// The document an account's patch is applied to.

import { z } from "zod";
import { Credential } from "../../../../../../daemon/endpoints/accounts/credential.js";

/**
 * The members of an account an edit may change, as one JSON
 * document: what a patch is applied to, and what its result must be
 * again — every member present, a member the patch takes away read
 * as absent or empty, and no member this type does not have. The
 * credential carries no key: the key is minted, never read.
 */
export const Editable = z.strictObject({
  /** The name, unique among accounts; another's is the edit's `in_use`. `null`, no name, which a container running under it forbids. */
  name: z.string().nullable(),
  /** How a client dials in as the account. `null`, no client does from then on, which a client connected as it now forbids. */
  credential: Credential.nullable(),
  /** What the account is for, in words. `null`, none. */
  description: z.string().nullable(),
  /** The roles it holds, by name. Empty, the account may do nothing. */
  roles: z.array(z.string()),
});
export type Editable = z.infer<typeof Editable>;
