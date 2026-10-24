// What a model thought of its tokens.

import { z } from "zod";

/**
 * An alternative the model considered at one position.
 */
export const TopLogprob = z.object({
  /** The token as text. */
  token: z.string(),
  /** The token's raw bytes. See `Logprob::bytes`. */
  bytes: z.array(z.number().int().nonnegative()).nullable(),
  /** Its log probability, when the provider reports one. */
  logprob: z.number().nullable(),
});
export type TopLogprob = z.infer<typeof TopLogprob>;

/**
 * One token, and what the model thought of it.
 */
export const Logprob = z.object({
  /** The token as text. */
  token: z.string(),
  /** The token's raw bytes, for tokens that are not valid UTF-8 on their own — a multi-byte character split across two tokens leaves each half unrepresentable as text. */
  bytes: z.array(z.number().int().nonnegative()).nullable(),
  /** The log probability the model assigned it. A decimal, not a float: these round-trip through JSON, and binary floating point loses exactly the low-order digits that make two providers' numbers comparable. */
  logprob: z.number(),
  /** What the model nearly chose instead, at this position. Empty when alternatives were not requested. */
  top_logprobs: z.array(TopLogprob),
});
export type Logprob = z.infer<typeof Logprob>;
