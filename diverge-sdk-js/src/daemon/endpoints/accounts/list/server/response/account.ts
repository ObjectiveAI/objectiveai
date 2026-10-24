// One account, as a list reports it.

import { z } from "zod";
import { Creator } from "../../../../../../daemon/creator/creator.js";
import { Credential } from "../../../../../../daemon/endpoints/accounts/credential.js";
import type { Keyed } from "../../../../../../shared/listing/keyed.js";

/**
 * One account: its name if it has one, its credential if it has one
 * — at least one of the two is there, and the credential carries no
 * key — what it is for, the roles it holds, whether a client is
 * connected as it now, its tags, when it was created and by whom. A
 * list sends them oldest created first.
 */
export const Account = z.object({
  /** The name, if the account has one: what a container's `account` names. Absent, none. */
  name: z.string().optional(),
  /** The credential, which carries no key, if the account has one. Absent, none. */
  credential: Credential.optional(),
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

/** Keyed by the credential's identity, else the name: an account has one or the other, and neither changes while it is listed. */
export const key: Keyed<Account> = (account) => account.credential?.identity ?? account.name ?? "";
