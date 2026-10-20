// One kind of chunk: `usage`.

import { z } from "zod";

/**
 * Token usage.
 *
 * Emitted as the loop goes rather than once at the end, so a caller
 * watches consumption grow instead of learning it after the fact. Each
 * chunk is a DELTA — every field is additive, so a caller that wants a
 * running total sums them.
 */
export const UsageChunk = z.object({
  /** Which kind this is. */
  type: z.literal("usage"),
  /** Tokens generated. */
  completion_tokens: z.number().int().nonnegative(),
  /** Prompt tokens consumed. */
  prompt_tokens: z.number().int().nonnegative(),
  /** The two above, summed. */
  total_tokens: z.number().int().nonnegative(),
  /** MCP's general field: any members. */
  _meta: z.record(z.string(), z.unknown()).optional(),
});
export type UsageChunk = z.infer<typeof UsageChunk>;
