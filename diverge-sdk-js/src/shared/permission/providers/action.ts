// What may be done to providers.

import { z } from "zod";

/**
 * An act over the daemon's providers.
 * Snake case on the wire: `"get"`, `"list"`, `"discover"`.
 *
 * - `get`: Read one, by identity.
 * - `list`: List them.
 * - `discover`: Discover the identities their discover listings hold, as `providers.discover` does.
 */
export const Action = z.enum(["get", "list", "discover"]);
export type Action = z.infer<typeof Action>;
