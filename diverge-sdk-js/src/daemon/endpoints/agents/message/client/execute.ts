// Performing the exchange: one request and one answer, or one request
// and what the agent's log takes of it.

import type { Handle } from "../../../../../wire/client/handle.js";
import type { Cancel } from "../../../../../daemon/client/cancel.js";
import * as one_shot from "../../../../../daemon/client/one_shot.js";
import { ExecuteStream, executeCancellable } from "../../../../../daemon/client/stream.js";
import * as request from "./request/index.js";
import * as response from "../server/response/index.js";

/**
 * Send a request that does not watch and read the daemon's one
 * answer, whichever it is — delivered, forbidden, a failure, or the
 * refusal the agent's image's document makes of it — as the response
 * frame, when the request's fate is known. The absence of an answer
 * is a `OneShotError`. A request that watches is `watch`'s: here only
 * its first answer would be read.
 */
export function execute(handle: Handle, frame: request.Frame): Promise<response.Frame> {
  return one_shot.execute(handle, frame, request.encode, response.decode);
}

/**
 * Send the request and hand back the stream of its answers and the
 * cancel that ends it. For a request that watches, the stream is the
 * items of the agent's log that are the request's — its echo first,
 * then everything the log takes to the end of that run — or the one
 * answer that it has none; for one that does not, the one answer. The
 * cancel is one channel request on the scope, after which the daemon
 * finishes the scope and what was sent before stands; the message
 * itself is not taken back.
 */
export function watch(handle: Handle, frame: request.Frame): Promise<[ExecuteStream<response.Frame>, Cancel]> {
  return executeCancellable(handle, frame, request.encode, response.decode);
}
