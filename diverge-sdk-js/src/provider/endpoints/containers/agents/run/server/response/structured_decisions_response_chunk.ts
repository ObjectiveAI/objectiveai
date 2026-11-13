// One kind of chunk: `structured_decisions_response`.

import { z } from "zod";
import { Answer } from "../../../../../../../shared/containers/structured_decisions/answer.js";

/**
 * The agent's answers to a structured decision: one answer for each
 * question, keyed by the question's id, each of the kind its question
 * is. Whole, never a delta. Nothing here ties it to the decision it
 * answers: the agent's log numbers the two alike.
 */
export const StructuredDecisionsResponseChunk = z.object({
  /** Which kind this is. */
  type: z.literal("structured_decisions_response"),
  /** Each question's answer, by the question's id. */
  decisions: z.record(z.string(), Answer),
  /** MCP's general field: any members. */
  _meta: z.record(z.string(), z.unknown()).optional(),
});
export type StructuredDecisionsResponseChunk = z.infer<typeof StructuredDecisionsResponseChunk>;
