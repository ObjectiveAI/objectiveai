// What narrows a list of the host's identities.

import { z } from "zod";

/**
 * The filter over the host's identities: every member optional, and
 * every one a condition an identity passes or does not. In a list
 * request it narrows what the daemon sends; in a grant it is how far
 * the grant reaches. A member that lists candidates matches an identity
 * that is any one of them; an empty list is absent.
 */
export const Filter = z.object({
  /** At any one of these issuers. */
  issuers: z.array(z.string()).optional(),
  /** Any one of these subjects, at whichever issuer. */
  subjects: z.array(z.string()).optional(),
  /** Every one of these among the identity's tags. */
  all_tags: z.array(z.string()).optional(),
  /** At least one of these among the identity's tags. */
  any_tags: z.array(z.string()).optional(),
});
export type Filter = z.infer<typeof Filter>;
