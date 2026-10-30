// Whose log a request reads.

import { z } from "zod";
import { Agent } from "../../../../reference/agent.js";
import { Tool } from "../../../../reference/tool.js";
import { Dependency } from "../../../../reference/dependency.js";

/**
 * Whose log: one member, naming the kind, over the reference its
 * family names it by. Later kinds — an account's commands, an
 * administrator's — join here.
 */
export const Subject = z.union([
  /** An agent's log. */
  z.strictObject({ agent: Agent }),
  /** A standalone tool's log. */
  z.strictObject({ standalone_tool: Tool }),
  /** A dependency tool's log. */
  z.strictObject({ dependency_tool: Dependency }),
]);
export type Subject = z.infer<typeof Subject>;
