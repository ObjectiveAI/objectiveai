// Performing the exchange: one request, one answer.

import type { Handle } from "../../../../../../../wire/client/handle.js";
import * as one_shot from "../../../../../../../daemon/client/one_shot.js";
import * as request from "./request/index.js";
import * as response from "../server/response/index.js";

/**
 * Send the request and read the daemon's one answer, whichever it
 * is — an answer or a refusal — as the response frame. The absence of an answer is
 * a `OneShotError`.
 */
export function execute(handle: Handle, frame: request.Frame): Promise<response.Frame> {
  return one_shot.execute(handle, frame, request.encode, response.decode);
}
