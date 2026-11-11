// Performing the exchange: one request, a stream ended at the outcome.

import type { Handle } from "../../../../../wire/client/handle.js";
import type { Cancel } from "../../../../../daemon/client/cancel.js";
import { ExecuteStream, executeCancellable } from "../../../../../daemon/client/stream.js";
import * as request from "./request/index.js";
import * as response from "../server/response/index.js";

/**
 * Send the request and hand back the stream of answers — `pending`,
 * then the outcome — and the cancel that ends the login before it: one
 * channel request on the scope, after which the daemon writes nothing
 * and finishes the scope.
 */
export function execute(handle: Handle, frame: request.Frame): Promise<[ExecuteStream<response.Frame>, Cancel]> {
  return executeCancellable(handle, frame, request.encode, response.decode);
}
