// How a provider's connection is made: it dials the daemon, or the daemon
// dials it.

import { z } from "zod";
import { Ip } from "./ip.js";

/**
 * Which end dials, and what that end needs: exactly one of the two for
 * every provider, tagged by `direction`, its members beside the
 * record's own. An incoming provider is admitted only from a source IP
 * its `ips` contain — any when absent, none when empty — and is never
 * dialled; an outgoing one is dialled at its `address`, and must answer
 * as the record's identity.
 */
export const Direction = z.discriminatedUnion("direction", [
  /** The provider dials the daemon. */
  z.object({ direction: z.literal("incoming"), ips: z.array(Ip).optional() }),
  /** The daemon dials the provider: `host:port`, or a URL with its own scheme. */
  z.object({ direction: z.literal("outgoing"), address: z.string() }),
]);
export type Direction = z.infer<typeof Direction>;
