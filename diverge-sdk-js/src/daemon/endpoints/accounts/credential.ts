// A credential a client dials in with.

import { z } from "zod";

/**
 * A credential a client dials in with: the identity it names, and
 * the one address it is accepted from, if any. The KEY is not here,
 * on any request or in any report: the daemon mints it when the
 * credential is made — on a create that gives one, or an edit that
 * sets one — answers it in that response, once, and never reports
 * it again; a client presents it byte for byte, and the daemon
 * knows it by the key alone. One credential per identity.
 */
export const Credential = z.object({
  /** Who a client presenting the key is: the identity it is served under, unless the account has a name, which is then the identity. */
  identity: z.string(),
  /** The one peer address the key is accepted from, as the OS reports it; absent, any address. */
  address: z.union([z.ipv4(), z.ipv6()]).optional(),
});
export type Credential = z.infer<typeof Credential>;
