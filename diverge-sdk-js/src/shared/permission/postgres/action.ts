// What is done with the database.

import { z } from "zod";

/**
 * The actions over the database, each the endpoint of the same name under `postgres`.
 * Snake case on the wire: `"get"`, `"list"`.
 *
 * - `get`: Read which database the daemon serves, a remote URL without its password.
 * - `list`: List the container connections open through it.
 */
export const Action = z.enum(["get", "list"]);
export type Action = z.infer<typeof Action>;
