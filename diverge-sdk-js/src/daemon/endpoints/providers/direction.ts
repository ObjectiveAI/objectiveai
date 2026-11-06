// How a provider's connection is made: it dials the daemon, or the daemon
// dials it.

import { z } from "zod";

/** One source address an incoming provider may dial from: a single IP, or a block of them. Tagged by `type`. */
export const Address = z.discriminatedUnion("type", [
  /** Exactly this IP, v4 or v6. */
  z.object({ type: z.literal("ip"), ip: z.string() }),
  /** Every IP in this block, as `10.0.0.0/24` or `2001:db8::/32`. */
  z.object({ type: z.literal("cidr"), cidr: z.string() }),
]);
export type Address = z.infer<typeof Address>;

/**
 * Which end dials, and what that end needs: exactly one of the two for
 * every provider, tagged by `direction`, its members beside the
 * record's own. An incoming provider is admitted only from a source its
 * `addresses` contain, any when absent, and is never dialled; an
 * outgoing one is dialled at its `address`, and must answer as the
 * record's identity.
 */
export const Direction = z.discriminatedUnion("direction", [
  /** The provider dials the daemon. */
  z.object({ direction: z.literal("incoming"), addresses: z.array(Address).optional() }),
  /** The daemon dials the provider: `host:port`, or a URL with its own scheme. */
  z.object({ direction: z.literal("outgoing"), address: z.string() }),
]);
export type Direction = z.infer<typeof Direction>;
