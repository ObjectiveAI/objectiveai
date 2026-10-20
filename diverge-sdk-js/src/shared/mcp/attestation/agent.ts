// An agent, as an attestation names one.

import { z } from "zod";
import { Image } from "../../../daemon/endpoints/agents/create/client/request/image.js";
import * as key from "../../../daemon/key/agent.js";

/**
 * An agent attested as the caller: the image it runs, and which
 * agent it is — its template, its index, its name when it has one —
 * flattened beside.
 */
export const Agent = z.object({
  /** The image the agent's container runs. */
  image: Image,
  ...key.Agent.shape,
});
export type Agent = z.infer<typeof Agent>;
