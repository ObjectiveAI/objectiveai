// One end of an edge: who sent, or who received.

import { z } from "zod";

/**
 * One party to an event, named once and for all by what never changes
 * and never by its name: an account by its identity, and an agent, a
 * standalone tool or a dependency tool by its template and its index
 * among those made from that template. Tagged by `kind`. A tool
 * connected from another daemon is never a party: it stands as the
 * party that connected it.
 */
export const Party = z.discriminatedUnion("kind", [
  /** An account, or an administrator, over an endpoint: its identity. */
  z.object({ kind: z.literal("account"), issuer: z.string(), subject: z.string() }),
  /** An agent, by its template and its index among the agents made from it. */
  z.object({ kind: z.literal("agent"), template: z.string(), index: z.number().int().nonnegative() }),
  /** A tool made from a template, by the template and its index among the tools made from it. */
  z.object({ kind: z.literal("standalone_tool"), template: z.string(), index: z.number().int().nonnegative() }),
  /** A dependency tool, by its dependency template and its index among the dependency tools made from it. */
  z.object({ kind: z.literal("dependency_tool"), template: z.string(), index: z.number().int().nonnegative() }),
]);
export type Party = z.infer<typeof Party>;
