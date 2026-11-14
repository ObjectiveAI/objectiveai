// One question's answer, as the log keeps it.

import { z } from "zod";
import { Noul } from "../../shared/containers/structured_decisions/answer.js";

/** A choice's answer, kept: the winning option, and the probabilities it won by. */
export const Choice = z.strictObject({
  /**
   * The option that won, by its value: the key with the highest
   * probability; where several share the highest, the one that sorts
   * first as text; with no probabilities at all, the empty string.
   */
  choice: z.string(),
  /** Each option's probability, keyed by its value. */
  probabilities: z.record(z.string(), z.number()),
});
export type Choice = z.infer<typeof Choice>;

/** A score's answer, kept: the score, and the probabilities it is the sum of. */
export const Score = z.strictObject({
  /**
   * The score: the sum, over the probabilities, of each probability
   * times its position, the first counting `0`. With no
   * probabilities, `0`.
   */
  score: z.number(),
  /** Each level's probability, in the levels' order. */
  probabilities: z.array(z.number()),
});
export type Score = z.infer<typeof Score>;

/**
 * The answer to one question as the agent's log keeps it: what the
 * agent's program said, and what the daemon made of it. The `choice`
 * and the `score` are never the program's: its answer has neither,
 * and the daemon computes both as it writes the item. Nothing is
 * checked against the question: probabilities that do not sum to one
 * and a score past the last level are computed as given.
 */
export const Answer = z.union([Score, Choice, Noul]);
export type Answer = z.infer<typeof Answer>;
