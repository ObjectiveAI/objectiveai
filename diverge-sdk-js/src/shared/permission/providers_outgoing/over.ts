// What is done to outgoing providers that exist.

import { z } from "zod";

/**
 * The actions over outgoing providers that exist, which a grant reaches as far as its `within` says.
 * Snake case on the wire: `"get"`, `"list"`, `"delete"`, `"edit"`.
 *
 * - `get`: Get one, as `providers.outgoing.get` does.
 * - `list`: List them, as `providers.outgoing.list` does; the list sends what the grant reaches.
 * - `delete`: Forget one, as `providers.outgoing.delete` does.
 * - `edit`: Replace one's mode, as `providers.outgoing.edit` does.
 */
export const Over = z.enum(["get", "list", "delete", "edit"]);
export type Over = z.infer<typeof Over>;
