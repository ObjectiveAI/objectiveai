// One question's answer.

import { z } from "zod";

/** A noul's answer: the one number. */
export const Noul = z.strictObject({
  /** The number. */
  noul: z.number(),
});
export type Noul = z.infer<typeof Noul>;

/** A choice's answer: a probability for each option, by the option's value. */
export const Choice = z.strictObject({
  /** Each option's probability, keyed by its value. */
  probabilities: z.record(z.string(), z.number()),
});
export type Choice = z.infer<typeof Choice>;

/** A score's answer: the score, and a probability for each level. */
export const Score = z.strictObject({
  /** The score. */
  score: z.number(),
  /** Each level's probability, in the levels' order. */
  probabilities: z.array(z.number()),
});
export type Score = z.infer<typeof Score>;

/**
 * The answer to one question, of the kind the question is. Untagged:
 * the three shapes take no member but their own, so a value is
 * exactly one of them, tried in the Rust's order. Every number is a
 * JSON number, never a string.
 */
export const Answer = z.union([Score, Choice, Noul]);
export type Answer = z.infer<typeof Answer>;
