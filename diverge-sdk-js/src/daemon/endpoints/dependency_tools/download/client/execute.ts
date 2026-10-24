// Performing the exchange: one request, the files streamed out.

import type { Handle } from "../../../../../wire/client/handle.js";
import * as stream from "../../../../../daemon/client/stream.js";
import * as request from "./request/index.js";
import * as response from "../server/response/index.js";

/**
 * Open the download and hand back its stream, without reading
 * anything: the daemon's first frame is the stream's, each item a
 * chunk with its bytes or one of the answers, and the finish ends
 * it. A finish with nothing before it is a directory with no file.
 * There is no cancel: a download is read to its end, or the handle
 * is closed.
 */
export function execute(handle: Handle, frame: request.Frame): Promise<stream.ExecuteStream<response.Item>> {
  return stream.execute(handle, frame, request.encode, response.decode);
}
