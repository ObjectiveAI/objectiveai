// What narrows a list of daemons.

import { z } from "zod";
import { Creator } from "../../../../../../creator/creator.js";
import { Identity } from "../../../../../agents/logs/server/response/identity.js";

/**
 * The filter over daemons: every member optional, and every one given
 * a condition a daemon passes or does not. In a list request it is
 * flattened into the request and narrows what the daemon sends; in a
 * grant it is read as a test. A filter with no member given passes
 * everything. A member that lists candidates matches any one of them;
 * an empty list is absent, and matches everything.
 */
export const Filter = z.object({
  /** Any one of these names. */
  names: z.array(z.string()).optional(),
  /** Linked through any one of these providers, as this daemon names them. */
  providers: z.array(Identity).optional(),
  /** Holding an account under any one of these names. Absent when empty. */
  accounts: z.array(z.string()).optional(),
  /** Whether this daemon holds a connection to it now, as any of its accounts: `true`, or not, `false`; absent, either. */
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
