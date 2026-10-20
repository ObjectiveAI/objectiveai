// How a provider dials in: the identity it has, and from where.

import { z } from "zod";

/**
 * A credential a provider dials in with: the identity it names, and
 * the one address it is accepted from, if any. The KEY is not here:
 * the daemon mints it when the credential is made — on an add, or an
 * edit that replaces it — answers it in that response, once, and
 * never reports it again. One credential per identity.
 */
export const Credential = z.object({
  /** Who a provider presenting the key is: its identity from then on. */
  identity: z.string(),
  /** The one peer address the key is accepted from; absent, any address. */
  address: z.union([z.ipv4(), z.ipv6()]).optional(),
});
export type Credential = z.infer<typeof Credential>;
