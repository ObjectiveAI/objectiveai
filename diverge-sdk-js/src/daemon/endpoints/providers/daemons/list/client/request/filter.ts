// What narrows a list of remote daemons.

import { z } from "zod";
import { Identity } from "../../../../../../../shared/identity.js";

/**
 * The filter over remote daemons: every member optional, and every
 * one given a condition a remote passes or does not. A filter with no
 * member given passes everything. A member that lists candidates
 * matches any one of them; an empty list is absent, and matches
 * everything.
 */
export const Filter = z.object({
  /** Any one of these names. */
  names: z.array(z.string()).optional(),
  /** Linked through any one of these providers, by identity. */
  providers: z.array(Identity).optional(),
  /** Whether this daemon holds a connection to it now: `true`, or not, `false`; absent, either. */
  connected: z.boolean().optional(),
});
export type Filter = z.infer<typeof Filter>;
