// What is done to connected tools that exist.

import { z } from "zod";

/**
 * The actions over connected tools that exist, which a grant reaches as far as its `within` says.
 * Snake case on the wire: `"get"`, `"list"`, `"edit"`, `"delete"`, `"attach"`, `"detach"`, `"logs"`.
 *
 * - `get`: Get one, as `connected_standalone_tools.get` does.
 * - `list`: List them, as `connected_standalone_tools.list` does; the list sends what the grant reaches.
 * - `edit`: Change one's name, as `connected_standalone_tools.edit` does.
 * - `delete`: Delete one, as `connected_standalone_tools.delete` does.
 * - `attach`: Attach one to an agent, as `attach_agent_instance` does, the agent judged by an `edit` grant over agents; or to every agent of a template, as `attach_agent_template` does, the template judged by an `attach` grant over agent templates.
 * - `detach`: Detach one from an agent, as `detach_agent_instance` does; or from every agent of a template, as `detach_agent_template` does — judged as `attach` is.
 * - `logs`: Read one's log, as `logs` does.
 */
export const Over = z.enum(["get", "list", "edit", "delete", "attach", "detach", "logs"]);
export type Over = z.infer<typeof Over>;
