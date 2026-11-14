// What is done to identities on the host.

import { z } from "zod";

/**
 * The actions over identities on the host, which a grant reaches as far as its `within` says.
 * Snake case on the wire: `"get"`, `"list"`, `"login"`.
 *
 * - `get`: Get one, as `identities.get` does.
 * - `list`: List them, as `identities.list` does; the list sends what the grant reaches.
 * - `login`: Sign one in again, replacing it, as `identities.login` does for one already on the host.
 */
export const Over = z.enum(["get", "list", "login"]);
export type Over = z.infer<typeof Over>;
