// What a client asks on the channels it opens on a connect.

import { tagged } from "../../../../../../wire/encode.js";
import * as call_tool from "../../../../../../shared/mcp/call_tool/request/request.js";
import * as list_resources from "../../../../../../shared/mcp/list_resources/request/request.js";
import * as list_tools from "../../../../../../shared/mcp/list_tools/request/request.js";
import * as notifications from "../../../../../../shared/mcp/notifications/request/request.js";
import * as read_resource from "../../../../../../shared/mcp/read_resource/request/request.js";

/**
 * What a client asks the daemon for on a connect, each on a channel
 * of its own: the disconnect, tag `0`, carrying nothing — nothing
 * answers on its channel; what answers is the scope's finish, and a
 * tool held by nothing else then stops — or one of the five MCP
 * exchanges, tag `1` to `5`, carrying the exchange's request as
 * `shared.mcp` writes it: the params as JSON and nothing in front,
 * no JSON-RPC envelope, the tag byte the whole of the distinction.
 * The daemon answers each on its channel with the exchange's frame.
 */
export type Frame =
  | "disconnect"
  | { readonly mcp_list_tools: list_tools.Request }
  | { readonly mcp_list_resources: list_resources.Request }
  | { readonly mcp_call_tool: call_tool.Request }
  | { readonly mcp_read_resource: read_resource.Request }
  | "mcp_notifications";

const DISCONNECT = 0;
const MCP_LIST_TOOLS = 1;
const MCP_LIST_RESOURCES = 2;
const MCP_CALL_TOOL = 3;
const MCP_READ_RESOURCE = 4;
const MCP_NOTIFICATIONS = 5;

/** The tag, then the exchange's request as it writes itself; the tag alone for the two that carry nothing. */
export function encode(frame: Frame): Uint8Array {
  if (frame === "disconnect") {
    return tagged(DISCONNECT);
  }
  if (frame === "mcp_notifications") {
    return tagged(MCP_NOTIFICATIONS, notifications.encode(null));
  }
  if ("mcp_list_tools" in frame) {
    return tagged(MCP_LIST_TOOLS, list_tools.encode(frame.mcp_list_tools));
  }
  if ("mcp_list_resources" in frame) {
    return tagged(MCP_LIST_RESOURCES, list_resources.encode(frame.mcp_list_resources));
  }
  if ("mcp_call_tool" in frame) {
    return tagged(MCP_CALL_TOOL, call_tool.encode(frame.mcp_call_tool));
  }
  return tagged(MCP_READ_RESOURCE, read_resource.encode(frame.mcp_read_resource));
}
