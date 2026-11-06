// A network IP, or a block of them.

import { z } from "zod";

/**
 * A network IP, or a block of them: what an incoming provider may dial
 * from, and what a remote daemon's route may be listed at. Tagged by
 * `type`. An IP is never a dial target: that is an address, a
 * `host:port` or a URL.
 */
export const Ip = z.discriminatedUnion("type", [
  /** Exactly this IP, v4 or v6. */
  z.object({ type: z.literal("ip"), ip: z.string() }),
  /** Every IP in this block, as `10.0.0.0/24` or `2001:db8::/32`. */
  z.object({ type: z.literal("cidr"), cidr: z.string() }),
]);
export type Ip = z.infer<typeof Ip>;
