// What may be done to providers.

import { z } from "zod";

/**
 * An act over the daemon's providers.
 * Snake case on the wire.
 *
 * - `get`: Read one, by identity.
 * - `list`: List them.
 * - `discover`: Discover the identities their discover listings hold, as `providers.discover` does.
 * - `listen`: Hear the messages sent to the daemon through them, as `daemons.listen` does.
 * - `add`: Add one.
 * - `edit`: Edit one.
 * - `delete`: Delete one.
 * - `enable`: Enable one.
 * - `disable`: Disable one: its runs taken down.
 * - `tag`: Put tags on one.
 * - `untag`: Take tags off one.
 */
export const Action = z.enum(["get", "list", "discover", "listen", "add", "edit", "delete", "enable", "disable", "tag", "untag"]);
export type Action = z.infer<typeof Action>;
