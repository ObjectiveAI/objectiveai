// One account, as a list reports it.

import { z } from "zod";
import { Creator } from "../../../../../../daemon/creator/creator.js";
import { Identity } from "../../../../../../shared/identity.js";
import type { Keyed } from "../../../../../../shared/listing/keyed.js";

/**
 * One account: its identity, what it is for, the roles it holds,
 * whether a client is connected as it now, its tags, when it was
 * created and by whom. A list sends them oldest created first.
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
  /** Its tags, sorted bytewise. Absent when empty. */
  tags: z.array(z.string()).optional(),
  /** When the create made it: an RFC 3339 timestamp in UTC. */
  created: z.iso.datetime({ offset: true }),
  /** Who created it, the direct maker. */
  creator: Creator,
});
export type Account = z.infer<typeof Account>;

/** Keyed by the identity, which never changes: another identity is another account. */
export const key: Keyed<Account> = (account) => account.identity;
