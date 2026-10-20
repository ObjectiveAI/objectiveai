// The container holding a connection.

import { z } from "zod";
import { Agent } from "../../reference/agent.js";
import { Dependency } from "../../reference/dependency.js";
import { Tool } from "../../reference/tool.js";

/**
 * Which container holds a database connection — which is to say,
 * which database scope it reaches: an agent or a tool on record,
 * named once and for all, or a dependency tool by its template and
 * whose it is. One object with one member, named for the family:
 * `{"agent":…}`, `{"tool":…}` or `{"dependency":…}`.
 */
export const Container = z.union([
  z.strictObject({ agent: Agent }),
  z.strictObject({ tool: Tool }),
  z.strictObject({ dependency: Dependency }),
]);
export type Container = z.infer<typeof Container>;
