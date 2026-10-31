// Whose log a request reads.

import { z } from "zod";
import { Tool } from "../../../../reference/tool.js";
import { Owner } from "../../../../reference/owner.js";

/**
 * Whose log: `type` naming the kind, and beside it the members of the
 * reference its family names it by — `{"type":"agent","name":"alice"}`,
 * `{"type":"account","issuer":…,"subject":…}`.
 */
export const Subject = z.union([
  /** An agent's log, by name. */
  z.strictObject({ type: z.literal("agent"), name: z.string() }),
  /** An agent's log, by template and index. */
  z.strictObject({ type: z.literal("agent"), template: z.string(), index: z.number().int().nonnegative() }),
  /** A standalone tool's log, by name. */
  z.strictObject({ type: z.literal("standalone_tool"), name: z.string() }),
  /** A standalone tool's log, by template and index. */
  z.strictObject({ type: z.literal("standalone_tool"), template: z.string(), index: z.number().int().nonnegative() }),
  /** A standalone tool's log, as another daemon names it. */
  z.strictObject({ type: z.literal("standalone_tool"), daemon: z.string(), tool: Tool }),
  /** A dependency tool's log, by its dependency template and its owner. */
  z.strictObject({ type: z.literal("dependency_tool"), template: z.string(), owner: Owner }),
  /** An account's command log, by its identity. */
  z.strictObject({ type: z.literal("account"), issuer: z.string(), subject: z.string() }),
  /** An administrator's command log, by the identity the daemon's configuration names. */
  z.strictObject({ type: z.literal("admin"), issuer: z.string(), subject: z.string() }),
]);
export type Subject = z.infer<typeof Subject>;
