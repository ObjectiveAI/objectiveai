// Which providers a grant reaches.

import { z } from "zod";
import { Identity } from "../../agent_log/identity.js";

/**
 * Which of the daemon's providers a grant reaches: by identity, by
 * the tags on their records, or by both. Read as a test: a provider
 * passes when it passes every member given.
 *
 * 1. `identities`: the provider is any one of these. Absent when
 *    empty, and then any provider.
 * 2. `all_tags`: the provider's record carries every one of these.
 * 3. `any_tags`: the provider's record carries any one of these.
 * 4. Together, all hold.
 * 5. No other member decodes: a grant must never widen by being
 *    misread.
 *
 * It is the `within` of every grant that acts over a provider or
 * through one: `providers`, `providers_daemons` and `images`.
 */
export const Reach = z.strictObject({
  /** Any one of these providers. Absent when empty, and then any provider. */
  identities: z.array(Identity).optional(),
  /** Every one of these among the provider's record's tags. Absent when empty, and then any tags. */
  all_tags: z.array(z.string()).optional(),
  /** Any one of these among the provider's record's tags; with `all_tags`, both hold. Absent when empty. */
  any_tags: z.array(z.string()).optional(),
});
export type Reach = z.infer<typeof Reach>;
