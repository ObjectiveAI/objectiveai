// One entry of a log, in its subject's kind.

import { z } from "zod";
import { ItemWrapper as AgentItem } from "../../../../agent_log/item_wrapper.js";
import { ItemWrapper as ToolItem } from "../../../../tool_log/item_wrapper.js";
import { ItemWrapper as CommandItem } from "../../../../command_log/item_wrapper.js";

/**
 * One entry of the log read: one member, naming the subject's kind,
 * over the entry as that kind's book keeps it. Every entry of one read
 * is of the kind its subject named.
 */
export const Entry = z.union([
  z.strictObject({ agent: AgentItem }),
  z.strictObject({ standalone_tool: ToolItem }),
  z.strictObject({ dependency_tool: ToolItem }),
  z.strictObject({ account: CommandItem }),
  z.strictObject({ admin: CommandItem }),
]);
export type Entry = z.infer<typeof Entry>;
