// Who a party is.

import { z } from "zod";

/**
 * A party's identity: the `(issuer, subject)` its token establishes —
 * a daemon's account, a provider, a daemon a provider serves. The
 * handshake's `Peer`, by the name the endpoints use for it. On the
 * wire, `{"issuer":…,"subject":…}`.
 */
export const Identity = z.object({
  /** The token's `iss`. */
  issuer: z.string(),
  /** The token's `sub`. */
  subject: z.string(),
});
export type Identity = z.infer<typeof Identity>;
