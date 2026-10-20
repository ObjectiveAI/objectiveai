// The part of a tool's origin that never changes.

import { z } from "zod";
import { Tool } from "../reference/tool.js";

/**
 * What a tool was made with, as much of it as is fixed for its life:
 * the template a created tool was made from, or the daemon and the
 * tool a connected tool joined. JSON-tagged by `kind`, `created` or
 * `connected`.
 */
export const Origin = z.discriminatedUnion("kind", [
  /** Made by a create, from a template. */
  z.object({ kind: z.literal("created"), template: z.string() }),
  /** Registered, from another daemon's tool. */
  z.object({ kind: z.literal("connected"), daemon: z.string(), account: z.string(), tool: Tool }),
]);
export type Origin = z.infer<typeof Origin>;
