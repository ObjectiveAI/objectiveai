// One structured decision, as it is handed to an agent.

import { z } from "zod";
import { ContentBlockSchema } from "@modelcontextprotocol/core";
import { Question } from "./question.js";

/**
 * One structured decision: the key, the content, and the questions,
 * as the agent's program is handed it and as the chunk that marks its
 * start carries it back.
 */
export const Decision = z.object({
  /** The caller's handle on the decision, as the daemon keyed it. */
  key: z.string(),
  /** The prompt's content, in order. */
  content: z.array(ContentBlockSchema),
  /** The questions, in order. */
  questions: z.array(Question),
});
export type Decision = z.infer<typeof Decision>;
