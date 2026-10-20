// What is done to roles that exist.

import { z } from "zod";

/**
 * The actions over roles that exist, which a grant reaches as far as its `within` says.
 * Snake case on the wire: `"get"`, `"list"`, `"delete"`, `"edit"`, `"grant"`.
 *
 * - `get`: Get one, as `roles.get` does.
 * - `list`: List them, as `roles.list` does; the list sends what the grant reaches.
 * - `delete`: Delete one, as `roles.delete` does.
 * - `edit`: Change one, as `roles.edit` does.
 * - `grant`: Name one in an account's `roles`, at the account's create or edit.
 */
export const Over = z.enum(["get", "list", "delete", "edit", "grant"]);
export type Over = z.infer<typeof Over>;
