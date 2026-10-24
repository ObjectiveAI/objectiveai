// Performing the exchange: one request, one answer waited for or
// cancelled.

import type { Handle } from "../../../../../wire/client/handle.js";
import * as pending from "../../../../../daemon/client/pending.js";
import * as request from "./request/index.js";
import * as response from "../server/response/index.js";

/**
 * Send the message and hand back its pending answer: `wait` reads
 * the daemon's one answer, whichever it is, when the message's fate
 * is known; `cancel` asks the daemon to take the message back, and
 * the answer then says which came first.
 */
export function execute(handle: Handle, frame: request.Frame): Promise<pending.Pending<response.Frame>> {
  return pending.execute(handle, frame, request.encode, response.decode);
}
