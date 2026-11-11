// Performing the exchange: one request, one answer.

import type { Handle } from "../../../../../wire/client/handle.js";
import * as one_shot from "../../../../../daemon/client/one_shot.js";
import * as request from "./request/index.js";
import * as response from "../server/response/index.js";

/**
 * Send the message and read the daemon's one answer, whichever it is
 * — delivered, forbidden, or a failure — as the response frame, when
 * the message's fate is known. The absence of an answer is a
 * `OneShotError`.
 */
export function execute(handle: Handle, frame: request.Frame): Promise<response.Frame> {
  return one_shot.execute(handle, frame, request.encode, response.decode);
}
