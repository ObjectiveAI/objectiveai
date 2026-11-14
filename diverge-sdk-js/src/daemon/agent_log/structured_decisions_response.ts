// An agent's answers to a structured decision, numbered and scored.

import { z } from "zod";
import { Answer } from "./answer.js";

/**
 * The answers the agent's program said, as the log keeps them: the
 * chunk's own members, each answer with what the daemon made of it —
 * a choice its `choice`, a score its `score` — and the id the log
 * gave them: the id of the decision they answer, where they are the
 * first after one, by the rules `UserStructuredDecisions` states.
 */
export const StructuredDecisionsResponse = z.object({
  /** The log's id for the response. */
  id: z.number().int().nonnegative(),
  /** Which kind this is. */
  type: z.literal("structured_decisions_response"),
  /** Each question's answer, by the question's id. */
  decisions: z.record(z.string(), Answer),
  /** MCP's general field: any members. */
  _meta: z.record(z.string(), z.unknown()).optional(),
});
export type StructuredDecisionsResponse = z.infer<typeof StructuredDecisionsResponse>;
