// What is done to, and through, providers' records.

import { z } from "zod";

/**
 * The actions over providers' records, which a grant reaches as far as its `within` says:
 * the providers whose records the `within` reaches, by their tags.
 * Snake case on the wire: `"get"`, `"list"`, `"discover"`, `"listen"`, `"message"`, `"edit"`, `"delete"`, `"enable"`, `"disable"`.
 *
 * - `get`: Read one, as `providers.get` does.
 * - `list`: List them, as `providers.list` does; the list sends what the grant reaches.
 * - `discover`: Discover the identities their discover listings hold, as `providers.discover` does; the listing holds only the routes through the providers the grant reaches.
 * - `listen`: Hear the messages sent to the daemon through them, as `daemons.listen` does; the daemon sends only those that came through the providers the grant reaches.
 * - `message`: Send messages through them, as `daemons.message` does.
 * - `edit`: Change one, as `providers.edit` does.
 * - `delete`: Delete one, as `providers.delete` does.
 * - `enable`: Enable one, as `providers.enable` does.
 * - `disable`: Disable one, its runs taken down, as `providers.disable` does.
 */
export const Over = z.enum(["get", "list", "discover", "listen", "message", "edit", "delete", "enable", "disable"]);
export type Over = z.infer<typeof Over>;
