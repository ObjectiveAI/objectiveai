// The connect, held: the channels a client opens on it.

import type { z } from "zod";
import type {
  CallToolResultSchema,
  ListResourcesResultSchema,
  ListToolsResultSchema,
  ReadResourceResultSchema,
} from "@modelcontextprotocol/core";
import { decodeServerFrame } from "../../../../../../wire/frame/server.js";
import type { Handle, Scope } from "../../../../../../wire/client/index.js";
import type { Image } from "../../../../../../daemon/endpoints/agents/create/client/request/image.js";
import { ChannelStream } from "../../../../../../provider/endpoints/containers/client/channel_stream.js";
import { OpenError } from "../../../../../../provider/endpoints/containers/client/error.js";
import { unary } from "../../../../../../provider/endpoints/containers/client/unary.js";
import type { ErrorData } from "../../../../../../shared/mcp/error_data.js";
import type { Notification } from "../../../../../../shared/mcp/notifications/response/notification.js";
import * as call_tool from "../../../../../../shared/mcp/call_tool/request/request.js";
import * as list_resources from "../../../../../../shared/mcp/list_resources/request/request.js";
import * as list_tools from "../../../../../../shared/mcp/list_tools/request/request.js";
import * as read_resource from "../../../../../../shared/mcp/read_resource/request/request.js";
import * as channel_request from "../channel_request/index.js";
import * as channel_response from "../../server/channel_response/index.js";
import type { Connected } from "../../server/response/connected.js";

/** How a connect ended. */
export type End = "finished" | "closed";

/** The tool's notifications, for as long as the channel lives. */
export type NotificationsStream = ChannelStream<Notification, ErrorData>;

/**
 * A connected tool, held open: what the daemon told of it, and the
 * channels this side opens on the scope — one per MCP exchange,
 * several in flight at once if wanted, and the disconnect. Nothing
 * happens on its own: dropping it sends nothing; `disconnect` asks
 * the daemon to finish the scope, and `wait` reads the end.
 */
export class ExecuteHandle {
  readonly #handle: Handle;
  readonly #scope: Scope;
  #end: Promise<End> | undefined;
  /** The image the tool's container runs. */
  readonly image: Image;
  /** The template the tool was made from, by id. */
  readonly template: string;

  constructor(handle: Handle, scope: Scope, connected: Connected) {
    this.#handle = handle;
    this.#scope = scope;
    this.image = connected.image;
    this.template = connected.template;
  }

  /** The scope's number. */
  get scope(): number {
    return this.#scope.scope;
  }

  /** `tools/list` against the tool. */
  listTools(params: list_tools.Request): Promise<z.infer<typeof ListToolsResultSchema>> {
    return unary(this.#handle, this.scope, channel_request.encode({ mcp_list_tools: params }), (payload) => {
      const frame = channel_response.mcp_list_tools.decode(payload);
      return "error" in frame ? { refusal: frame.error } : { item: frame.result };
    });
  }

  /** `resources/list` against the tool. */
  listResources(params: list_resources.Request): Promise<z.infer<typeof ListResourcesResultSchema>> {
    return unary(this.#handle, this.scope, channel_request.encode({ mcp_list_resources: params }), (payload) => {
      const frame = channel_response.mcp_list_resources.decode(payload);
      return "error" in frame ? { refusal: frame.error } : { item: frame.result };
    });
  }

  /** `tools/call` against the tool. */
  callTool(params: call_tool.Request): Promise<z.infer<typeof CallToolResultSchema>> {
    return unary(this.#handle, this.scope, channel_request.encode({ mcp_call_tool: params }), (payload) => {
      const frame = channel_response.mcp_call_tool.decode(payload);
      return "error" in frame ? { refusal: frame.error } : { item: frame.result };
    });
  }

  /** `resources/read` against the tool. */
  readResource(params: read_resource.Request): Promise<z.infer<typeof ReadResourceResultSchema>> {
    return unary(this.#handle, this.scope, channel_request.encode({ mcp_read_resource: params }), (payload) => {
      const frame = channel_response.mcp_read_resource.decode(payload);
      return "error" in frame ? { refusal: frame.error } : { item: frame.result };
    });
  }

  /** Everything the tool says on its own account, for as long as the channel lives. */
  async notifications(): Promise<NotificationsStream> {
    let channel;
    try {
      channel = await this.#handle.sendChannelRequest(this.scope, channel_request.encode("mcp_notifications"));
    } catch (cause) {
      throw new OpenError("send", cause);
    }
    return new ChannelStream<Notification, ErrorData>(channel.responseReceiver, (payload) => {
      const frame = channel_response.mcp_notifications.decode(payload);
      return "error" in frame ? { refusal: frame.error } : { item: frame.notification };
    });
  }

  /** Leave the tool. Nothing answers on the channel this opens; what answers is the scope's own finish, on `wait`. */
  async disconnect(): Promise<void> {
    try {
      await this.#handle.sendChannelRequest(this.scope, channel_request.encode("disconnect"));
    } catch (cause) {
      throw new OpenError("send", cause);
    }
  }

  /**
   * The connection's end: `finished` at the scope's finish — the
   * disconnect answered, or the tool's run ended — or `closed` where
   * the connection ended first. Resolves once, and answers the same
   * way again after.
   */
  wait(): Promise<End> {
    this.#end ??= this.#read();
    return this.#end;
  }

  async #read(): Promise<End> {
    for (;;) {
      const bytes = await this.#scope.responseReceiver.next();
      if (bytes === undefined) {
        return "closed";
      }
      let envelope;
      try {
        envelope = decodeServerFrame(bytes);
      } catch {
        return "closed";
      }
      if (envelope.kind === "response_finish") {
        return "finished";
      }
    }
  }
}
