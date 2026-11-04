// One identity discovered, and the providers it was seen on.

import { z } from "zod";
import { Identity } from "../../../../../../shared/identity.js";
import type { Keyed } from "../../../../../../shared/listing/keyed.js";

/**
 * One identity discoverable through the daemon's connected providers,
 * and every connected provider it is discoverable on, in order — by
 * issuer, then subject — never empty. A `daemons.connect` remote
 * reaching it is `issuer` as its `issuer`, and one link per provider:
 * `provider` that provider, `daemon` this identity.
 */
export const Discovered = z.object({
  /** The `iss` claim the identity's connections were authorized under. */
  issuer: z.string(),
  /** The `sub` claim. */
  subject: z.string(),
  /** Every connected provider it is discoverable on, in order. */
  providers: z.array(Identity),
});
export type Discovered = z.infer<typeof Discovered>;

/** Keyed by the identity. */
export const key: Keyed<Discovered> = (discovered) => ({ issuer: discovered.issuer, subject: discovered.subject });
