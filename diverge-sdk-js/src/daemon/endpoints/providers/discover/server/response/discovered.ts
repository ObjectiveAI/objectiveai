// One identity discovered, and the routes it was seen on.

import { z } from "zod";
import type { Keyed } from "../../../../../../shared/listing/keyed.js";
import { Via } from "./via.js";

/**
 * One identity discoverable through the daemon's connected providers,
 * a route for every connected provider it is discoverable on — the
 * provider, the IP, and the address when the provider dialled it — in
 * order of provider, never empty, and every distinct description
 * those providers list it with, in order: equal descriptions are one
 * entry, and none is the empty array. A `daemons.connect` remote
 * reaching it is `issuer` as its `issuer`, and one link per provider:
 * `provider` that provider, `daemon` this identity.
 */
export const Discovered = z.object({
  /** The `iss` claim the identity's connections were authorized under. */
  issuer: z.string(),
  /** The `sub` claim. */
  subject: z.string(),
  /** Every route to it, in order of provider. */
  providers: z.array(Via),
  /** Every distinct description it is listed with, in order: one trimmed line of 1 to 132 characters each. */
  descriptions: z.array(z.string().min(1).max(132)),
});
export type Discovered = z.infer<typeof Discovered>;

/** Keyed by the identity. */
export const key: Keyed<Discovered> = (discovered) => ({ issuer: discovered.issuer, subject: discovered.subject });
