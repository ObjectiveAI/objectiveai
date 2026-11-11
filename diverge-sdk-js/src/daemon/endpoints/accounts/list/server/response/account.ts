// One account, as a list reports it.

import { z } from "zod";
import { Creator } from "../../../../../../daemon/creator/creator.js";
import { Identity } from "../../../../../../shared/identity.js";
import type { Keyed } from "../../../../../../shared/listing/keyed.js";

/**
 * One account: its identity, what it is for, the roles it holds,
 * whether a client is connected as it now, its tags, when it was
 * created and by whom, and whether the daemon's configuration names it
 * an administrator. A list sends them oldest created first, the
 * administrators the configuration names with no account of their own
 * after them — `admin` true, no `created`, no `creator`.
 */
export const Account = z.object({
  /** The identity the account is: the `(issuer, subject)` a client's token establishes. */
  identity: Identity,
  /** What the account is for, as its create or edit said. Absent, none. */
  description: z.string().optional(),
  /** The roles it holds, by name, sorted bytewise. Absent when empty. */
  roles: z.array(z.string()).optional(),
  /** Whether a client is connected as it now. */
  connected: z.boolean(),
  /** When that last changed; absent for one never connected as. */
  last_connected: z.iso.datetime({ offset: true }).optional(),
  /** Its tags, sorted bytewise. Absent when empty. */
  tags: z.array(z.string()).optional(),
  /** When the create made it: an RFC 3339 timestamp in UTC. Absent for an administrator with no account of its own. */
  created: z.iso.datetime({ offset: true }).optional(),
  /** Who created it, the direct maker. Absent for an administrator with no account of its own. */
  creator: Creator.optional(),
  /** Whether the daemon's configuration names the identity an administrator. Absent, false. */
  admin: z.boolean().optional(),
});
export type Account = z.infer<typeof Account>;

/** Keyed by the identity, which never changes: another identity is another account. */
export const key: Keyed<Account> = (account) => account.identity;
