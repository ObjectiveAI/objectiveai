// What is done to accounts that exist.

import { z } from "zod";

/**
 * The actions over accounts that exist, which a grant reaches as far as its `within` says.
 * Snake case on the wire: `"get"`, `"list"`, `"delete"`, `"edit"`, `"logs"`.
 *
 * - `get`: Get one, as `accounts.get` does.
 * - `list`: List them, as `accounts.list` does; the list sends what the grant reaches.
 * - `delete`: Delete one, as `accounts.delete` does.
 * - `edit`: Change one, as `accounts.edit` does.
 * - `logs`: Read one's command log, as `logs` does.
 */
export const Over = z.enum(["get", "list", "delete", "edit", "logs"]);
export type Over = z.infer<typeof Over>;
