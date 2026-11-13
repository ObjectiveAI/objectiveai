// What an agent that makes structured decisions says of itself.

import { z } from "zod";

import { Modalities } from "./modalities.js";
import { Modality } from "./modality.js";

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

/**
 * What a decision carries that an image does not take, by where it
 * carries it: each list in the order image, audio, file.
 */
export const Lacking = z.object({
  /** Carried by the prompt — the content and the instructions — and not taken there. */
  prompt: z.array(Modality),
  /** Carried by the choices — the options' descriptions and the levels — and not taken there. */
  choice: z.array(Modality),
});
export type Lacking = z.infer<typeof Lacking>;
