// Who a request came from.

import { z } from "zod";
import { Agent } from "./agent.js";

/**
 * Who a request came from, as the daemon attests it on the request's
 * `_meta`: an agent, by its image and its key, or an unbrokered
 * client by its identity. Tagged by `kind`.
 */
export const Caller = z.discriminatedUnion("kind", [
  /** An agent of the daemon's. */
  z.object({ kind: z.literal("agent"), ...Agent.shape }),
  /** A client dialled in unbrokered, by the identity its credential names. */
  z.object({ kind: z.literal("unbrokered"), identity: z.string() }),
]);
export type Caller = z.infer<typeof Caller>;
