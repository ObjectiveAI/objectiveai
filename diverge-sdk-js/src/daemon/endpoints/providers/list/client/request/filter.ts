// What narrows a list of providers.

import { z } from "zod";
import { Identity } from "../../../../../../shared/identity.js";

/**
 * The filter over providers: every member optional, and every one
 * given a condition a provider passes or does not. A filter with no
 * member given passes everything. A member that lists candidates
 * matches any one of them; an empty list is absent, and matches
 * everything.
 */
export const Filter = z.object({
  /** Any one of these identities. */
  identities: z.array(Identity).optional(),
  /** Whether the daemon's status with it is `connected`: `true`, or not, `false`; absent, either. */
  connected: z.boolean().optional(),
});
export type Filter = z.infer<typeof Filter>;
