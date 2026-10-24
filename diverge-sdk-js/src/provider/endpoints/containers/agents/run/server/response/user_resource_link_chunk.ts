// One kind of chunk: `user_resource_link`.

import { z } from "zod";
import { ResourceSchema } from "@modelcontextprotocol/core";

/**
 * A resource link of a delivered message.
 *
 * One block of the message, whole, at the position the message landed:
 * between the tool responses it was folded in behind, or opening the
 * next turn when the assistant had already finished, or first of all,
 * for the message a run started on. A message's parts arrive
 * consecutively, in the message's order, each under its `key`; a
 * message is its run of parts under one key. The stream's order is the
 * only statement of WHERE; a part is the statement of THAT, and of
 * WHICH.
 *
 * The payload is MCP's own `Resource`, flattened, so what the caller
 * said is expressed in the same vocabulary a tool would use to return
 * it and the model would use to say it — one content model across the
 * whole loop rather than one per direction.
 */
export const UserResourceLinkChunk = z.object({
  /** Which kind this is. */
  type: z.literal("user_resource_link"),
  /** Which message of the agent's inbox the part belongs to, as the daemon keyed it. */
  key: z.string(),
  /** The content, MCP's own, flattened in: its `type` is this chunk's. */
  ...ResourceSchema.shape,
});
export type UserResourceLinkChunk = z.infer<typeof UserResourceLinkChunk>;
