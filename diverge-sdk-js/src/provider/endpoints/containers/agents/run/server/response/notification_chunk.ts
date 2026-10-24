// One kind of chunk: `notification`.

import { z } from "zod";

/**
 * Something the loop has to say that is not part of its output.
 *
 * A warning, a retry, a degraded mode, a failure — anything a provider
 * wants a caller to know about the run itself rather than about what
 * the agent produced.
 *
 * In-band rather than a transport signal, because a loop keeps going
 * after most of these and can fail after producing output. Ending the
 * stream without saying why would leave a caller holding a partial
 * result and no way to tell it apart from a complete one.
 */
export const NotificationChunk = z.object({
  /** Which kind this is. */
  type: z.literal("notification"),
  /** Whether the run ends here. `true` is the loop saying it is over and this is why. `false` is it saying something and carrying on — a warning, a retry, a degraded mode, a failure it recovered from. Fatal rather than merely wrong, because "wrong" is not a question a caller can act on and "over" is. A provider that hits an error and retries past it has not failed, and a caller told otherwise would abandon a run that was still going. Nothing here says what a provider must send either way. What is worth mentioning, and what it can recover from, is the provider's to decide. */
  is_fatal: z.boolean(),
  /** The message or details, as an arbitrary JSON value — providers report failures in shapes we do not get to dictate, and flattening one into a string would discard the structure a caller needs to act on it. */
  message: z.json(),
  /** MCP's general field: any members. */
  _meta: z.record(z.string(), z.unknown()).optional(),
});
export type NotificationChunk = z.infer<typeof NotificationChunk>;
