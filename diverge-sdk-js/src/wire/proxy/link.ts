// The link's two commands: how it stands, and dialling again.

import { WIRE_RESET, WIRE_STATUS } from "./commands.js";
import { WireStatus } from "./status.js";
import type { Transport } from "./transport.js";

/**
 * How the app's link to the daemon stands. When nothing has been
 * dialled since the link was made or reset, the app's Rust dials
 * first, so this is also how a page connects; a link that is down
 * stays down, and this never redials it.
 */
export async function status(transport: Transport): Promise<WireStatus> {
  return WireStatus.parse(await transport.invoke(WIRE_STATUS, {}));
}

/**
 * Put the link back to not yet dialled: the next use — a send, or
 * `status` — dials. A page's one way to dial again after the
 * connection ended; every receiver the old connection fed has already
 * closed.
 */
export async function reset(transport: Transport): Promise<void> {
  await transport.invoke(WIRE_RESET, {});
}
