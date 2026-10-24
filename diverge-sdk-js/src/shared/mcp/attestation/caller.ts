// Who a request came from.

import { z } from "zod";
import { Identity } from "../../identity.js";
import { Agent } from "./agent.js";

/**
 * Who a request came from, as the daemon attests it on the request's
 * `_meta`: an agent, by its image and its key, or a client by the
 * identity its handshake established — as every `Creator.client`
 * names the same client. Tagged by `kind`.
 */
export const Caller = z.discriminatedUnion("kind", [
  /** An agent of the daemon's. */
  z.object({ kind: z.literal("agent"), ...Agent.shape }),
  /** A client of the daemon's, calling over a connect: the account the connection is. */
  z.object({ kind: z.literal("client"), identity: Identity }),
]);
export type Caller = z.infer<typeof Caller>;
