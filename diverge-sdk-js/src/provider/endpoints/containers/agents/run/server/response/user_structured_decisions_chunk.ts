// One kind of chunk: `user_structured_decisions`.

import { z } from "zod";
import { Decision } from "../../../../../../../shared/containers/structured_decisions/decision.js";

/**
 * A structured decision of the caller's, landing: the whole decision
 * — its key, its content, its questions — as the run it starts takes
 * it, and the first chunk of that run. One chunk, where a chat
 * message is one per content block: a decision is asked whole.
 */
export const UserStructuredDecisionsChunk = z.object({
  /** Which kind this is. */
  type: z.literal("user_structured_decisions"),
  /** The decision, flattened in: `key`, `content`, `questions`. */
  ...Decision.shape,
  /** MCP's general field: any members. */
  _meta: z.record(z.string(), z.unknown()).optional(),
});
export type UserStructuredDecisionsChunk = z.infer<typeof UserStructuredDecisionsChunk>;
