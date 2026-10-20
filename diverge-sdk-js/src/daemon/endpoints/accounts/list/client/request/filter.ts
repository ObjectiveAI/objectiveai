// What narrows a list of accounts.

import { z } from "zod";
import { Creator } from "../../../../../creator/creator.js";

/**
 * The filter over accounts: every member optional, and every one given
 * a condition an account passes or does not. In a list request it is
 * flattened into the request and narrows what the daemon sends; in a
 * grant it is read as a test. A filter with no member given passes
 * everything. A member that lists candidates matches any one of them;
 * an empty list is absent, and matches everything.
 */
export const Filter = z.object({
  /** Any one of these names. */
  names: z.array(z.string()).optional(),
  /** With a credential naming any one of these identities. */
  identities: z.array(z.string()).optional(),
  /** Whether the account has a name: `true`, or none, `false`; absent, either. */
  named: z.boolean().optional(),
  /** Whether the account has a credential: `true`, or none, `false`; absent, either. */
  credentialed: z.boolean().optional(),
  /** Holding any one of these roles, by name. */
  roles: z.array(z.string()).optional(),
  /** Whether a client is connected as it now: `true`, or none, `false`; absent, either. */
  connected: z.boolean().optional(),
  /** Made by any one of these, directly. Absent when empty, and then made by anybody. */
  creators: z.array(Creator).optional(),
  /** Every one of these among the thing's tags. Absent when empty, and then any tags. */
  all_tags: z.array(z.string()).optional(),
  /** Any one of these among the thing's tags; with `all_tags`, both hold. Absent when empty. */
  any_tags: z.array(z.string()).optional(),
  /** The earliest `created` to list, inclusive; absent, no earliest. */
  created_from: z.iso.datetime({ offset: true }).optional(),
  /** The latest `created` to list, inclusive; absent, no latest. */
  created_to: z.iso.datetime({ offset: true }).optional(),
});
export type Filter = z.infer<typeof Filter>;
