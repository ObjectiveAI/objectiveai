// What is done to agent templates that exist.

import { z } from "zod";

/**
 * The actions over agent templates that exist, which a grant reaches as far as its `within` says.
 * Snake case on the wire: `"get"`, `"attach"`, `"delete"`, `"list"`.
 *
 * - `get`: Get one, as `agents.templates.get` does.
 * - `attach`: Attach a tool to every agent of one, or detach one, as `standalone_tools.attach_agent_template` and `detach_agent_template` do; the tool is judged by an `attach` or a `detach` grant over tools.
 * - `delete`: Delete one, as `agents.templates.delete` does.
 * - `list`: List them, as `agents.templates.list` does; the list sends what the grant reaches.
 */
export const Over = z.enum(["get", "attach", "delete", "list"]);
export type Over = z.infer<typeof Over>;
