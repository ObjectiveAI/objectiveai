import { invokeCliRequest } from "../../../viewer/invoke";
import type { CliCommandRequest } from "../request";

/**
 * Viewer executor: posts the request to the host viewer bridge and streams
 * the `cli_command` events back. Thin wrapper over the existing
 * [`invokeCliRequest`] transport. Takes no construction options.
 */
export class ViewerCommandExecutor {
  execute(request: CliCommandRequest): AsyncIterable<unknown> {
    return invokeCliRequest(request);
  }
}
