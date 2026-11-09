// What is done to, and through, remote daemons' records.

import { z } from "zod";

/**
 * The actions over remote daemons' records, which a grant reaches as far as its `within` says.
 * Snake case on the wire: `"get"`, `"list"`, `"edit"`, `"delete"`, `"connect"`, `"relay"`, `"oci"`.
 *
 * - `get`: Read one, as `daemons.get` does.
 * - `list`: List them, as `daemons.list` does; the list sends what the grant reaches.
 * - `edit`: Change one, as `daemons.edit` does.
 * - `delete`: Delete one, as `daemons.delete` does.
 * - `connect`: Hold a connected tool on one, as `connected_standalone_tools.create` does.
 * - `relay`: Relay any request to one: the first name of a request's `relay`.
 * - `oci`: Ask one for images, as `images.oci` does; the daemon asks only the remote daemons whose records the grant reaches.
 */
export const Over = z.enum(["get", "list", "edit", "delete", "connect", "relay", "oci"]);
export type Over = z.infer<typeof Over>;
