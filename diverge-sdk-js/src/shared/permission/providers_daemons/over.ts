// What is done through providers, to the daemons connected to them.

import { z } from "zod";

/**
 * The actions through providers, which a grant reaches as far as its `within` says:
 * the providers whose records the `within` reaches, by their tags.
 * Snake case on the wire: `"list"`, `"listen"`, `"message"`.
 *
 * - `list`: List the identities their listings hold, as `providers.daemons.list` does; the listing holds only the routes through the providers the grant reaches.
 * - `listen`: Hear the messages sent to the daemon through them, as `providers.daemons.listen` does; the daemon sends only those that came through the providers the grant reaches.
 * - `message`: Send messages through them, as `providers.daemons.message` does; a message is sent through no provider the grant does not reach.
 */
export const Over = z.enum(["list", "listen", "message"]);
export type Over = z.infer<typeof Over>;
