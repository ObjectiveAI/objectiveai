// What may be done to providers.

import { z } from "zod";

/**
 * An act over the daemon's providers.
 * Snake case on the wire: `"get"`, `"list"`.
 *
 * - `get`: Read one, by identity.
 * - `list`: List them.
 */
export const Action = z.enum(["get", "list"]);
export type Action = z.infer<typeof Action>;
