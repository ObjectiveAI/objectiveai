// Performing the exchange: one request, every file's content on the
// channels the daemon opens, one answer.

import type { Handle } from "../../../../../wire/client/handle.js";
import * as upload from "../../../../../daemon/client/upload.js";
import * as request from "./request/index.js";
import * as channel_request from "../server/channel_request/index.js";
import * as response from "../server/response/index.js";

/**
 * Send the request, answer every content channel the daemon opens
 * from `source` — asked with the file's path as the request named
 * it, or `null` for a file upload — and hand back the daemon's one
 * answer, whichever it is: see `daemon.client.upload` for the
 * exchange.
 */
export function execute(handle: Handle, frame: request.Frame, source: upload.Source): Promise<response.Frame> {
  return upload.execute(handle, frame, request.encode, ask, response.decode, source);
}

/** The content ask, read: which file. */
function ask(payload: Uint8Array): string | null {
  return channel_request.decode(payload).path ?? null;
}
