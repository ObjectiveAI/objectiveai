// What is done to vault keys that exist.

import { z } from "zod";

/**
 * The actions over vault keys that exist, which a grant reaches as far as its `within` says.
 * Snake case on the wire: `"get"`, `"list"`, `"insert"`, `"delete"`, `"use"`.
 *
 * - `get`: Get one, with its value, as `vault.get` does.
 * - `list`: List them, as `vault.list` does; the list sends what the grant reaches, and never a value.
 * - `insert`: Append a value to one, or replace one of its values, as `vault.insert` does over a key the vault holds.
 * - `delete`: Delete one, or one of its values, as `vault.delete` does.
 * - `use`: Bind one to a secret of an agent's or a tool's, in the `secrets` of its create or edit. The key must be one the vault holds, with at least one value.
 */
export const Over = z.enum(["get", "list", "insert", "delete", "use"]);
export type Over = z.infer<typeof Over>;
