// What is done to volumes that exist.

import { z } from "zod";

/**
 * The actions over volumes that exist, which a grant reaches as far as its `within` says.
 * Snake case on the wire: `"get"`, `"list"`, `"delete"`, `"edit"`, `"stat"`, `"download"`, `"upload"`, `"transfer"`, `"mount"`, `"filetree"`.
 *
 * - `get`: Get one, as `volumes.get` does.
 * - `list`: List them, as `volumes.list` does; the list sends what the grant reaches.
 * - `delete`: Delete one, as `volumes.delete` does.
 * - `edit`: Change one's size or mode, as `volumes.edit` does.
 * - `stat`: Walk one, as `volumes.stat` does.
 * - `download`: Send the client files out of one, as `volumes.download` does.
 * - `upload`: Put files into one, as `volumes.upload` does, and as a transfer landing in one does.
 * - `transfer`: Copy files out of one, as `volumes.transfer` does; where they land is judged by its own grant.
 * - `mount`: Name one in a container's mounts — a volume mount of the provider it is pinned to, or a FUSE mount of a file or a directory in it — at the container's create or edit, beside the grant over the container.
 * - `filetree`: See one's tree, as `volumes.filetree` does.
 */
export const Over = z.enum(["get", "list", "delete", "edit", "stat", "download", "upload", "transfer", "mount", "filetree"]);
export type Over = z.infer<typeof Over>;
