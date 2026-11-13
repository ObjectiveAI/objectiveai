// One option of a choice.

import { z } from "zod";
import { ContentBlockSchema } from "@modelcontextprotocol/core";

/**
 * One option of a choice question: its value, and perhaps a
 * description. No other member.
 */
export const ChoiceOption = z.strictObject({
  /** The option's name, text alone: what its probability is keyed by in the answer. */
  value: z.string(),
  /** What the option means, as content parts, in order. Absent, the value says it all. */
  description: z.array(ContentBlockSchema).optional(),
});
export type ChoiceOption = z.infer<typeof ChoiceOption>;
