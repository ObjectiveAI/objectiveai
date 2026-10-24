// What is done to incoming credentials that exist.

import { z } from "zod";

/**
 * The actions over incoming credentials that exist, which a grant reaches as far as its `within` says.
 * Snake case on the wire: `"get"`, `"list"`, `"delete"`, `"edit"`.
 *
 * - `get`: Get one, as `providers.incoming.get` does.
 * - `list`: List them, as `providers.incoming.list` does; the list sends what the grant reaches.
 * - `delete`: Take one out, as `providers.incoming.delete` does.
 * - `edit`: Replace one, as `providers.incoming.edit` does.
 */
export const Over = z.enum(["get", "list", "delete", "edit"]);
export type Over = z.infer<typeof Over>;
