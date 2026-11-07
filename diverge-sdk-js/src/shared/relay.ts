// A request served through other daemons: the member every request of
// the daemon's wire may carry.

import { z } from "zod";
import { DaemonId } from "../daemon/endpoints/daemons/id.js";

/**
 * Where a request is served: through other daemons, by the IDs of this
 * daemon's records of them, and of each next daemon's records in turn,
 * see `daemons`. Every request carries this as an optional member,
 * `relay`, an array of IDs; absent or empty, the request is served by
 * the daemon it was sent to. A daemon that reads a request with `relay`
 * takes the first ID off, judges that the caller
 * may relay to that record — the daemons grant's `relay` reaching it, and
 * nothing else — connects to the remote as the account the record names,
 * and puts the request there with the rest as its `relay`, carrying every
 * frame both ways until the remote finishes. The remote judges the next
 * ID, and the request, as that account's. An ID is the hash of what its
 * record reaches and as whom, so no daemon along the route can re-point
 * a hop: it holds the record the ID names, or none, and refuses. A relay
 * the daemon will not make is answered as the request's own `forbidden`
 * or `error`.
 */
export const Relay = z.array(DaemonId);
export type Relay = z.infer<typeof Relay>;
