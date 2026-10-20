// A request served by another daemon: the member every request of
// the daemon's wire may carry.

import { z } from "zod";

/**
 * Where a request is served: on another daemon, as one of the
 * accounts this daemon holds on it — a record of
 * `providers.daemons` and one of its accounts, both by name. Every
 * request of the daemon's wire carries this as an optional member,
 * `relay`; absent, the request is served by the daemon it was sent
 * to. A daemon that reads a request with `relay` does not serve it:
 * it takes or opens its connection to that daemon as that account,
 * puts the request there with this member taken out, and carries
 * every frame both ways until the remote finishes, reading none of
 * it. A relay the daemon will not make is answered as the request's
 * own `forbidden` or `error`.
 */
export const Relay = z.object({
  /** The daemon, by the name of its record. */
  daemon: z.string(),
  /** The account on it to act as, by the name the record's accounts give it. */
  account: z.string(),
});
export type Relay = z.infer<typeof Relay>;
