// What an agent that makes structured decisions says of itself.

import { z } from "zod";

import { Modalities } from "./modalities.js";

/**
 * How an agent's program makes structured decisions: two members,
 * both required, and no other. Stated by an agent's image, in the
 * frontmatter's `structured_decisions`. Nothing reads it yet.
 */
export const StructuredDecisions = z.strictObject({
  /** What the prompt of a decision may carry beside text. */
  prompt_modalities: Modalities,
  /** What a choice of a decision may carry beside text. */
  choice_modalities: Modalities,
});
export type StructuredDecisions = z.infer<typeof StructuredDecisions>;
