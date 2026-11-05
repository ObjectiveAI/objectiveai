// A request served through other daemons: the member every request of
// the daemon's wire may carry.

import { z } from "zod";

/**
 * Where a request is served: through other daemons, by the names of this
 * daemon's records of them, see `daemons`. Every request carries this as
 * an optional member, `relay`, an array of names; absent or empty, the
 * request is served by the daemon it was sent to. A daemon that reads a
 * request with `relay` takes the first name off, judges that the caller
 * may relay to that record — the daemons grant's `relay` reaching it, and
 * nothing else — connects to the remote as the account the record names,
 * and puts the request there with the rest as its `relay`, carrying every
 * frame both ways until the remote finishes. The remote judges the next
 * name, and the request, as that account's. A relay the daemon will not
 * make is answered as the request's own `forbidden` or `error`.
 */
export const Relay = z.array(z.string());
export type Relay = z.infer<typeof Relay>;
