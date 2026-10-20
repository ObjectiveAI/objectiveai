// What is done to tools that exist.

import { z } from "zod";

/**
 * The actions over tools that exist, which a grant reaches as far as its `within` says.
 * Snake case on the wire: `"get"`, `"edit"`, `"attach"`, `"detach"`, `"delete"`, `"list"`, `"download"`, `"upload"`, `"transfer"`, `"filetree"`, `"connect"`.
 *
 * - `get`: Get one, as `standalone_tools.get` does.
 * - `edit`: Change one, as `standalone_tools.edit` does.
 * - `attach`: Attach one to an agent, as `attach_agent_instance` does, the agent judged by an `edit` grant over agents; or to every agent of a template, as `attach_agent_template` does, the template judged by an `attach` grant over agent templates.
 * - `detach`: Detach one from an agent, as `detach_agent_instance` does; or from every agent of a template, as `detach_agent_template` does — judged as `attach` is.
 * - `delete`: Delete one, as `standalone_tools.delete` does.
 * - `list`: List them, as `standalone_tools.list` does; the list sends what the grant reaches.
 * - `download`: Send the client files out of one, as `standalone_tools.download` does.
 * - `upload`: Put files into one, as `standalone_tools.upload` does, and as a transfer landing in one does.
 * - `transfer`: Copy files out of one, as `standalone_tools.transfer` does; where they land is judged by its own grant.
 * - `filetree`: Watch one's container whole, as `standalone_tools.filetree` does.
 * - `connect`: Serve one to another daemon, as `standalone_tools.connect` does.
 */
export const Over = z.enum(["get", "edit", "attach", "detach", "delete", "list", "download", "upload", "transfer", "filetree", "connect"]);
export type Over = z.infer<typeof Over>;
