// One identity listed, and the routes it was seen on.

import { z } from "zod";
import { Description } from "../../../../../../../shared/description.js";
import type { Keyed } from "../../../../../../../shared/listing/keyed.js";
import { Via } from "./via.js";

/**
 * One identity announced on the daemon's connected providers,
 * a route for every connected provider it is announced on — the
 * provider, the IP, and the address when the provider dialled it — in
 * order of provider, never empty, and every distinct announcement
 * those providers list it with, in order: equal announcements are one
 * entry, and the array is never empty, since every provider that
 * lists it lists it with one. A `daemons.connect` remote
 * reaching it is `issuer` as its `issuer`, and one link per provider:
 * `provider` that provider, `daemon` this identity.
 */
export const Listed = z.object({
  /** The `iss` claim the identity's connections were authorized under. */
  issuer: z.string(),
  /** The `sub` claim. */
  subject: z.string(),
  /** Every route to it, in order of provider. */
  providers: z.array(Via),
  /** Every distinct announcement it is listed with, in order. */
  announcements: z.array(Description),
});
export type Listed = z.infer<typeof Listed>;

/** Keyed by the identity. */
export const key: Keyed<Listed> = (listed) => ({ issuer: listed.issuer, subject: listed.subject });
