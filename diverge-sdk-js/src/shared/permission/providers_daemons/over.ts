// What is done to daemons that exist.

import { z } from "zod";

/**
 * The actions over daemons that exist, which a grant reaches as far as its `within` says.
 * Snake case on the wire: `"get"`, `"list"`, `"delete"`, `"edit"`, `"connect"`, `"relay"`.
 *
 * - `get`: Get one, as `providers.daemons.get` does.
 * - `list`: List them, as `providers.daemons.list` does; the list sends what the grant reaches.
 * - `delete`: Forget one, as `providers.daemons.delete` does.
 * - `edit`: Replace one's links, as `providers.daemons.edit` does; and add, rotate and forget its accounts, as `providers.daemons.accounts` does.
 * - `connect`: Reach a tool of the daemon's as one of the record's accounts, as `standalone_tools.register` names one. Judged against the grant's `accounts` as well as its `within`.
 * - `relay`: Put a request to the daemon as one of the record's accounts — any request carrying a `relay` naming the record and the account. Judged against the grant's `accounts` as well as its `within`.
 */
export const Over = z.enum(["get", "list", "delete", "edit", "connect", "relay"]);
export type Over = z.infer<typeof Over>;
