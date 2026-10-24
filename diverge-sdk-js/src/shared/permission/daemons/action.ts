// What may be done through remote daemons.

import { z } from "zod";

/**
 * An act over the daemons this daemon reaches.
 * Snake case on the wire: `"get"`, `"list"`, `"connect"`, `"relay"`.
 *
 * - `get`: Read one, by name.
 * - `list`: List them.
 * - `connect`: Connect a tool of one's to this daemon.
 * - `relay`: Relay a request to one, served there.
 */
export const Action = z.enum(["get", "list", "connect", "relay"]);
export type Action = z.infer<typeof Action>;
