// What passed between a sender and a receiver.

import { z } from "zod";

/**
 * The three things the daemon records passing from one party to
 * another: a `message` enqueued to an agent, a `call` into a tool, a
 * `read` of a tool's resource. Snake case on the wire.
 */
export const Action = z.enum(["message", "call", "read"]);
export type Action = z.infer<typeof Action>;
