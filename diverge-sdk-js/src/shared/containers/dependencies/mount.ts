// An agent's path, mounted into its tool.

import { z } from "zod";

/**
 * A path of the agent container, mounted at a path of the tool
 * container over FUSE: source before destination. Which of the two
 * it is — a file or a directory — is which list of the `Template` it
 * is on. The caller serves the agent container's path live and makes
 * the mount in the tool container as a FUSE mount answered from it,
 * so the two containers share the path for as long as both run.
 */
export const Mount = z.object({
  /** Where it is in the agent container, as path components from the root: no component empty, `.` or `..`, and not the root. */
  agent_path: z.array(z.string()),
  /** Where it appears in the tool container, as path components from the root: not the root, no other mount's, inside no other mount's. */
  tool_path: z.array(z.string()),
});
export type Mount = z.infer<typeof Mount>;
