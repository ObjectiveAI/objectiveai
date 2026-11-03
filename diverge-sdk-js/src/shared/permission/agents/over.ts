// What is done to agents that exist.

import { z } from "zod";

/**
 * The actions over agents that exist, which a grant reaches as far as its `within` says.
 * Snake case on the wire: `"get"`, `"delete"`, `"edit"`, `"message"`, `"logs"`, `"list"`, `"download"`, `"upload"`, `"transfer"`, `"filetree"`, `"exec"`, `"frontend"`.
 *
 * - `get`: Get one, as `agents.get` does.
 * - `delete`: Delete one, as `agents.delete` does.
 * - `edit`: Change one, as `agents.edit` does.
 * - `message`: Send one a message, as `agents.message` does.
 * - `logs`: Read one's log, as `agents.logs` does.
 * - `list`: List them, as `agents.list` does; the list sends what the grant reaches.
 * - `download`: Send the client files out of one, as `agents.download` does.
 * - `upload`: Put files into one, as `agents.upload` does, and as a transfer landing in one does.
 * - `transfer`: Copy files out of one, as `agents.transfer` does; where they land is judged by its own grant.
 * - `filetree`: Watch one's container whole, as `agents.filetree` does.
 * - `exec`: Run a command in one's container — any program it holds, as the container's own user — as `agents.exec` does.
 * - `frontend`: Send HTTP requests to the frontend its program serves, as `agents.frontend` does.
 */
export const Over = z.enum(["get", "delete", "edit", "message", "logs", "list", "download", "upload", "transfer", "filetree", "exec", "frontend"]);
export type Over = z.infer<typeof Over>;
