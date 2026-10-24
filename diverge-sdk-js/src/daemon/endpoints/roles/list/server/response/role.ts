// One role, as a list reports it.

import { z } from "zod";
import { Creator } from "../../../../../../daemon/creator/creator.js";
import { Reference } from "../../../../../../daemon/endpoints/accounts/reference.js";
import { Grant } from "../../../../../../daemon/grant/grant.js";
import type { Keyed } from "../../../../../../shared/listing/keyed.js";

/**
 * One role: its name, what it is for, its grants, the accounts that
 * hold it, its tags, when it was created and by whom. A list sends
 * them oldest created first.
 */
export const Role = z.object({
  /** The name. */
  name: z.string(),
  /** What the role is for, as its create or edit said. Absent, none. */
  description: z.string().optional(),
  /** The grants, as the create or the last edit gave them, in that order. Absent when empty, and then the role allows nothing. */
  grants: z.array(Grant).optional(),
  /** The accounts that hold it, in no order, each by its name when it has one and otherwise by its credential. Absent when empty. */
  accounts: z.array(Reference).optional(),
  /** Its tags, sorted bytewise. Absent when empty. */
  tags: z.array(z.string()).optional(),
  /** When the create made it: an RFC 3339 timestamp in UTC. */
  created: z.iso.datetime({ offset: true }),
  /** Who created it, the direct maker. */
  creator: Creator,
});
export type Role = z.infer<typeof Role>;

/** Keyed by the name. */
export const key: Keyed<Role> = (role) => role.name;
