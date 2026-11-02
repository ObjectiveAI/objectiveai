// Performing the exchange: one request, the command's stream.

import type { Handle } from "../../../../../wire/client/handle.js";
import type { Cancel } from "../../../../../daemon/client/cancel.js";
import { ExecuteStream, executeCancellable } from "../../../../../daemon/client/stream.js";
import * as request from "./request/index.js";
import * as response from "../server/response/index.js";

/**
 * Send the request and hand back the command's stream — each item a
 * `response.Item`, stdout and stderr with their bytes — and the cancel
 * that kills it. Leaving the stream early abandons it, which kills the
 * command too.
 */
export function execute(handle: Handle, frame: request.Frame): Promise<[ExecuteStream<response.Item>, Cancel]> {
  return executeCancellable(handle, frame, request.encode, response.decode);
}
