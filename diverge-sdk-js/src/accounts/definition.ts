// The two shapes an account takes.

import { z } from "zod";
import { Credential } from "./credential.js";

/** An account with a name, and a credential if any client may dial in as it. */
export const Named = z.object({
  /** The name, unique among accounts; compared and not read. */
  name: z.string(),
  /** How a client dials in as the account, if any client may. Absent, none does. */
  credential: Credential.optional(),
});
export type Named = z.infer<typeof Named>;

/** An account without a name: a client dials in as it, and no container runs under it. */
export const Unnamed = z.object({
  /** How a client dials in as the account. */
  credential: Credential,
});
export type Unnamed = z.infer<typeof Unnamed>;

/**
 * The two shapes an account takes, and between them the rule that
 * an account has a name, a credential, or both, and never neither.
 * Untagged JSON, told apart by whether `name` is there — `Named` is
 * tried first — and not strict, since a create flattens it beside
 * its own members; an object with neither does not decode.
 */
export const Definition = z.union([Named, Unnamed]);
export type Definition = z.infer<typeof Definition>;
