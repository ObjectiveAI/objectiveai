// What is done to, and through, remote daemons' records.

import { z } from "zod";

/**
 * The actions over remote daemons' records, which a grant reaches as far as its `within` says.
 * Snake case on the wire: `"get"`, `"list"`, `"edit"`, `"delete"`, `"relay"`.
 * The first four are over the record itself; `relay` is the one permission to USE the daemon the record reaches, whatever the use.
 *
 * - `get`: Read one, as `daemons.get` does.
 * - `list`: List them, as `daemons.list` does; the list sends what the grant reaches.
 * - `edit`: Change one, as `daemons.edit` does.
 * - `delete`: Delete one, as `daemons.delete` does.
 * - `relay`: Use the daemon the record reaches, as the account the record names. One action for every use: relay any request to it, the first ID of a request's `relay`; hold a connected tool on it, as `connected_standalone_tools.create` does; name a volume on it in a container's FUSE mounts; ask it for images, as `images.read` does, the daemon asking only the remote daemons whose records the grant reaches. What the daemon reached then allows that account is that daemon's to judge, by its own grants.
 */
export const Over = z.enum(["get", "list", "edit", "delete", "relay"]);
export type Over = z.infer<typeof Over>;
