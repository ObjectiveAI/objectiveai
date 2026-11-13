// A structured decision a caller sent, numbered, with who sent it.

import { z } from "zod";
import { Creator } from "../../daemon/creator/creator.js";
import { UserStructuredDecisionsChunk } from "../../provider/endpoints/containers/agents/run/server/response/user_structured_decisions_chunk.js";

/**
 * A structured decision as it landed: the chunk, flattened in, the
 * id the log gave it, and who sent it.
 *
 * The id is the log's, a whole number, and is how a
 * `StructuredDecisionsResponse` is read against the decision it
 * answers. For one agent's log: a decision takes one more than the
 * highest id before it, `0` where there is none; the first response
 * after a decision no response has answered takes that decision's
 * id; any other response takes one more than the highest before it.
 * So `decision, response, response, decision, response` is `0, 0, 1,
 * 2, 2`.
 */
export const UserStructuredDecisions = z.intersection(
  z.object({
    /** The log's id for the decision. */
    id: z.number().int().nonnegative(),
    /** Who sent it, where the daemon knows. Absent for a key the daemon did not send. */
    sender: Creator.optional(),
  }),
  UserStructuredDecisionsChunk,
);
export type UserStructuredDecisions = z.infer<typeof UserStructuredDecisions>;
