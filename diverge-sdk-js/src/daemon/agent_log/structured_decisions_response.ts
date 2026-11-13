// An agent's answers to a structured decision, numbered.

import { z } from "zod";
import { StructuredDecisionsResponseChunk } from "../../provider/endpoints/containers/agents/run/server/response/structured_decisions_response_chunk.js";

/**
 * The answers as the agent's program said them, flattened in, and the
 * id the log gave them: the id of the decision they answer, where
 * they are the first after one, by the rules `UserStructuredDecisions`
 * states.
 */
export const StructuredDecisionsResponse = z.intersection(
  z.object({
    /** The log's id for the response. */
    id: z.number().int().nonnegative(),
  }),
  StructuredDecisionsResponseChunk,
);
export type StructuredDecisionsResponse = z.infer<typeof StructuredDecisionsResponse>;
