// A request served by another daemon: the member every request of
// the daemon's wire may carry.

import { z } from "zod";

/**
 * Where a request is served: on another daemon — one of the remote
 * daemons of this daemon's configuration, see `providers.daemons`, by
 * name. Every request of the daemon's wire carries this as an optional
 * member, `relay`; absent, the request is served by the daemon it was
 * sent to. A daemon that reads a request with `relay` does not serve
 * it: it checks that the caller may relay — the daemons grant's
 * `relay` action — takes or opens its connection to that daemon, made
 * as its own identity at the remote's issuer, puts the request there
 * with this member taken out, and carries every frame both ways until
 * the remote finishes, reading none of it. The remote serves it as
 * this daemon's account there. A relay the daemon will not make is
 * answered as the request's own `forbidden` or `error`.
 */
export const Relay = z.object({
  /** The remote daemon, by its name in the configuration. */
  daemon: z.string(),
});
export type Relay = z.infer<typeof Relay>;
