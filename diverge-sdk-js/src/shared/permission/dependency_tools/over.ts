// What is done to dependency tools that exist.

import { z } from "zod";

/**
 * The actions over dependency tools that exist, which a grant reaches as far as its `within` says.
 * Snake case on the wire: `"get"`, `"delete"`, `"list"`, `"download"`, `"upload"`, `"transfer"`, `"filetree"`.
 *
 * - `get`: Get one, as `dependency_tools.get` does.
 * - `delete`: Delete every tool of a template, as `dependency_tools.delete` does.
 * - `list`: List them, as `dependency_tools.list` does; the list sends what the grant reaches.
 * - `download`: Send the client files out of one, running, as `dependency_tools.download` does.
 * - `upload`: Put files into one, running, as `dependency_tools.upload` does, and as a transfer landing in one does.
 * - `transfer`: Copy files out of one, running, as `dependency_tools.transfer` does; where they land is judged by its own grant.
 * - `filetree`: Watch one's container whole, as `dependency_tools.filetree` does.
 */
export const Over = z.enum(["get", "delete", "list", "download", "upload", "transfer", "filetree"]);
export type Over = z.infer<typeof Over>;
