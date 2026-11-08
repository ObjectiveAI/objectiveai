// Performing the exchange: one request, one answer waited for, or
// none once cancelled in time.

import type { Handle } from "../../../../../../wire/client/handle.js";
import * as pending from "../../../../../../daemon/client/pending.js";
import * as request from "./request/index.js";
import * as response from "../server/response/index.js";

/**
 * Send the message and hand back its pending answer: `wait` reads the
 * daemon's one answer when the message's fate is known, or `null` for
 * the finish with no answer before it — a cancel in time; `cancel` asks
 * the daemon to take the message back.
 */
export function execute(handle: Handle, frame: request.Frame): Promise<pending.Pending<response.Frame>> {
  return pending.execute(handle, frame, request.encode, response.decode);
}
