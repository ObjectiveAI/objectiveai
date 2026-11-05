// What narrows a list of remote daemons' records.

import { z } from "zod";
import { Creator } from "../../../../../creator/creator.js";

/**
 * The filter over remote daemons' records: every member optional, and
 * every one a condition a record passes or does not. In a list request it
 * narrows what the daemon sends; in a grant it is how far the grant
 * reaches. A member that lists candidates matches any one of them; an
 * empty list is absent.
 */
export const Filter = z.object({
  /** Any one of these names. */
  names: z.array(z.string()).optional(),
  /** The remote at any one of these issuers. */
  issuers: z.array(z.string()).optional(),
  /** Added by any one of these. */
  creators: z.array(Creator).optional(),
  /** Whether this daemon holds a connection to it now: `true`, or not, `false`; absent, either. */
  connected: z.boolean().optional(),
  /** Every one of these among its tags. */
  all_tags: z.array(z.string()).optional(),
  /** At least one of these among its tags. */
  any_tags: z.array(z.string()).optional(),
  /** Added at or after this. */
  created_from: z.string().optional(),
  /** Added at or before this. */
  created_to: z.string().optional(),
});
export type Filter = z.infer<typeof Filter>;
