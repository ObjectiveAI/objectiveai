// Whose log a request reads.

import { z } from "zod";
import { Agent } from "../../../../reference/agent.js";
import { Tool } from "../../../../reference/tool.js";
import { Dependency } from "../../../../reference/dependency.js";
import { Identity } from "../../../../../shared/identity.js";

/**
 * Whose log: one member, naming the kind, over the reference its
 * family names it by.
 */
export const Subject = z.union([
  /** An agent's log. */
  z.strictObject({ agent: Agent }),
  /** A standalone tool's log. */
  z.strictObject({ standalone_tool: Tool }),
  /** A dependency tool's log. */
  z.strictObject({ dependency_tool: Dependency }),
  /** An account's command log, by its identity. */
  z.strictObject({ account: Identity }),
  /** An administrator's command log, by the identity the daemon's configuration names. */
  z.strictObject({ admin: Identity }),
]);
export type Subject = z.infer<typeof Subject>;
