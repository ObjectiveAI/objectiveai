// One user part, with who sent it.

import { z } from "zod";
import { Creator } from "../../daemon/creator/creator.js";
import { AgenticLoopChunk } from "../../provider/endpoints/containers/agents/run/server/response/chunk.js";

/**
 * One user part the run streamed — a text, an image, an audio, a
 * resource or a resource link of a message that landed — with who sent
 * the message beside it. The part's own members are flattened here, so
 * the item reads as the chunk it is with `sender` added; which message
 * the part belongs to, its own `key` says.
 */
export const User = z.intersection(
  z.object({
    /** Who sent the message the part is of: the client, an agent or a tool. Nothing but a user part has a sender. */
    sender: Creator,
  }),
  AgenticLoopChunk,
);
export type User = z.infer<typeof User>;
