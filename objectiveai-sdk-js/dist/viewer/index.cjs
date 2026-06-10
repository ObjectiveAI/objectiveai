'use strict';

var zod = require('zod');

var __defProp = Object.defineProperty;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __esm = (fn, res) => function __init() {
  return fn && (res = (0, fn[__getOwnPropNames(fn)[0]])(fn = 0)), res;
};
var __export = (target, all) => {
  for (var name in all)
    __defProp(target, name, { get: all[name], enumerable: true });
};
var init_tslib_es6 = __esm({
  "../node_modules/@tauri-apps/api/external/tslib/tslib.es6.js"() {
  }
});

// ../node_modules/@tauri-apps/api/core.js
function transformCallback(callback, once2 = false) {
  return window.__TAURI_INTERNALS__.transformCallback(callback, once2);
}
async function invoke(cmd, args = {}, options) {
  return window.__TAURI_INTERNALS__.invoke(cmd, args, options);
}
var init_core = __esm({
  "../node_modules/@tauri-apps/api/core.js"() {
    init_tslib_es6();
  }
});

// ../node_modules/@tauri-apps/api/event.js
var event_exports = {};
__export(event_exports, {
  TauriEvent: () => TauriEvent,
  emit: () => emit,
  emitTo: () => emitTo,
  listen: () => listen,
  once: () => once
});
async function _unlisten(event, eventId) {
  window.__TAURI_EVENT_PLUGIN_INTERNALS__.unregisterListener(event, eventId);
  await invoke("plugin:event|unlisten", {
    event,
    eventId
  });
}
async function listen(event, handler, options) {
  var _a;
  const target = typeof (options === null || options === void 0 ? void 0 : options.target) === "string" ? { kind: "AnyLabel", label: options.target } : (_a = options === null || options === void 0 ? void 0 : options.target) !== null && _a !== void 0 ? _a : { kind: "Any" };
  return invoke("plugin:event|listen", {
    event,
    target,
    handler: transformCallback(handler)
  }).then((eventId) => {
    return async () => _unlisten(event, eventId);
  });
}
async function once(event, handler, options) {
  return listen(event, (eventData) => {
    void _unlisten(event, eventData.id);
    handler(eventData);
  }, options);
}
async function emit(event, payload) {
  await invoke("plugin:event|emit", {
    event,
    payload
  });
}
async function emitTo(target, event, payload) {
  const eventTarget = typeof target === "string" ? { kind: "AnyLabel", label: target } : target;
  await invoke("plugin:event|emit_to", {
    target: eventTarget,
    event,
    payload
  });
}
var TauriEvent;
var init_event = __esm({
  "../node_modules/@tauri-apps/api/event.js"() {
    init_core();
    (function(TauriEvent2) {
      TauriEvent2["WINDOW_RESIZED"] = "tauri://resize";
      TauriEvent2["WINDOW_MOVED"] = "tauri://move";
      TauriEvent2["WINDOW_CLOSE_REQUESTED"] = "tauri://close-requested";
      TauriEvent2["WINDOW_DESTROYED"] = "tauri://destroyed";
      TauriEvent2["WINDOW_FOCUS"] = "tauri://focus";
      TauriEvent2["WINDOW_BLUR"] = "tauri://blur";
      TauriEvent2["WINDOW_SCALE_FACTOR_CHANGED"] = "tauri://scale-change";
      TauriEvent2["WINDOW_THEME_CHANGED"] = "tauri://theme-changed";
      TauriEvent2["WINDOW_CREATED"] = "tauri://window-created";
      TauriEvent2["WEBVIEW_CREATED"] = "tauri://webview-created";
      TauriEvent2["DRAG_ENTER"] = "tauri://drag-enter";
      TauriEvent2["DRAG_OVER"] = "tauri://drag-over";
      TauriEvent2["DRAG_DROP"] = "tauri://drag-drop";
      TauriEvent2["DRAG_LEAVE"] = "tauri://drag-leave";
    })(TauriEvent || (TauriEvent = {}));
  }
});
var JsonValueSchema = zod.z.union([
  zod.z.string(),
  zod.z.number(),
  zod.z.boolean(),
  zod.z.null(),
  zod.z.array(zod.z.lazy(() => JsonValueSchema)),
  zod.z.record(zod.z.string(), zod.z.lazy(() => JsonValueSchema))
]);

// src/viewer/event.ts
var ViewerEventSchema = zod.z.union([zod.z.object({
  destination: zod.z.string(),
  sub_type: zod.z.string(),
  type: zod.z.literal("inbound"),
  value: JsonValueSchema
}).describe("Host \u2192 iframe. Carries data into the plugin (the existing\npath). `sub_type` is the snake_case discriminator the plugin\nlistens on (built-ins: `agent_completions` /\n`functions_executions` / `functions_inventions_recursive` /\n`laboratories_executions`; plugins: whatever they declared in\ntheir manifest's `viewer_routes[i].type`).").meta({ "variantTitle": "Inbound" }), zod.z.object({
  destination: zod.z.string(),
  type: zod.z.literal("cli_command"),
  value: JsonValueSchema
}).describe('Host \u2192 iframe. One stdout JSONL line from an objectiveai cli\nbinary the host spawned for an `invokeCli` this iframe\nstarted, terminated by a synthetic `{"type":"end"}` line. No\nsub_type \u2014 a single invocation produces a single stream of\nlines.').meta({ "variantTitle": "CliCommand" })]).describe("Every event the viewer emits to the JS side. Serde-tagged on\n`type` so the JS bridge can pattern-match and decide how to\nrepackage each variant for the destination iframe.\n\n`destination` is `\"objectiveai\"` for built-in events, or the\nplugin's repository name otherwise. For `CliCommand` it's the\nrepository name of whichever iframe invoked the CLI \u2014 the bridge\nderives it from `MessageEvent.source`, the plugin author never\nsets it.").meta({ title: "viewer.Event" });

// src/viewer/cliStream.ts
var CliStream = class {
  constructor(source, schema) {
    this.source = source;
    this.schema = schema;
  }
  async *[Symbol.asyncIterator]() {
    for await (const line of this.source) {
      if (isEndMarker(line)) {
        continue;
      }
      yield parseUnwrapping(this.schema, line);
    }
  }
  /** Collect every remaining item. */
  async toArray() {
    const items = [];
    for await (const item of this) {
      items.push(item);
    }
    return items;
  }
  /**
   * Resolve the first item and discard the rest of the stream
   * (releasing the underlying message listener). `undefined` when the
   * stream ends without yielding — a unary command that printed
   * nothing before the end marker.
   */
  async first() {
    const iter = this[Symbol.asyncIterator]();
    try {
      const result = await iter.next();
      return result.done ? void 0 : result.value;
    } finally {
      await iter.return?.(void 0);
    }
  }
};
function isEndMarker(line) {
  return typeof line === "object" && line !== null && line.type === "end";
}
function parseUnwrapping(schema, value) {
  let current = value;
  for (; ; ) {
    const result = schema.safeParse(current);
    if (result.success) return result.data;
    if (current !== null && typeof current === "object" && !Array.isArray(current)) {
      const keys = Object.keys(current);
      if (keys.length === 1) {
        current = current[keys[0]];
        continue;
      }
    }
    return schema.parse(value);
  }
}

// src/viewer/invoke.ts
function cliCommandIterable(post) {
  return {
    [Symbol.asyncIterator]() {
      const queue = [];
      let resolveNext = null;
      let done = false;
      let cleaned = false;
      const onMessage = (event) => {
        const msg = event.data;
        if (!msg || typeof msg !== "object") return;
        if (msg.kind !== "plugin-event" || msg.type !== "cli_command") return;
        queue.push(msg.value);
        if (typeof msg.value === "object" && msg.value !== null && msg.value.type === "end") {
          done = true;
        }
        if (resolveNext) {
          const r = resolveNext;
          resolveNext = null;
          r();
        }
      };
      const cleanup = () => {
        if (cleaned) return;
        cleaned = true;
        if (typeof window !== "undefined") {
          window.removeEventListener("message", onMessage);
        }
      };
      if (typeof window === "undefined" || window.parent === window) {
        console.warn(
          "@objectiveai/sdk/viewer: cli invocation outside an iframe; no host present, the iterator will yield nothing."
        );
        done = true;
      } else {
        window.addEventListener("message", onMessage);
        post();
      }
      return {
        async next() {
          while (queue.length === 0 && !done) {
            await new Promise((r) => {
              resolveNext = r;
            });
          }
          if (queue.length > 0) {
            return { value: queue.shift(), done: false };
          }
          cleanup();
          return { value: void 0, done: true };
        },
        async return() {
          cleanup();
          return { value: void 0, done: true };
        }
      };
    }
  };
}
function invokeCliRequest(request) {
  return cliCommandIterable(() => {
    window.parent.postMessage({ kind: "cli-execute", request }, "*");
  });
}
var RemotePathSchema = zod.z.union([zod.z.object({
  commit: zod.z.string(),
  owner: zod.z.string(),
  remote: zod.z.literal("github"),
  repository: zod.z.string()
}).meta({ "variantTitle": "Github" }), zod.z.object({
  commit: zod.z.string(),
  owner: zod.z.string(),
  remote: zod.z.literal("filesystem"),
  repository: zod.z.string()
}).meta({ "variantTitle": "Filesystem" }), zod.z.object({
  name: zod.z.string(),
  remote: zod.z.literal("mock")
}).meta({ "variantTitle": "Mock" })]).meta({ title: "RemotePath" });

// src/agent/getAgentResponse.ts
var AgentGetAgentResponseSchema = RemotePathSchema.and(zod.z.object({})).describe("Response containing a single Agent with creation timestamp.").meta({ title: "agent.GetAgentResponse" });
var CliErrorTypeSchema = zod.z.literal("error").describe('Single-variant discriminator for [`Error`]\'s `type` field.\nAlways `"error"` on the wire.').meta({ title: "cli.ErrorType" });
var CliLevelSchema = zod.z.enum(["trace", "debug", "info", "warn", "error"]).describe("Severity matching the conventions used by bunyan / pino / `log` crate\nJSON encoders. `fatal` is encoded separately on [`Error`].").meta({ title: "cli.Level" });

// src/cli/error.ts
var CliErrorSchema = zod.z.object({
  fatal: zod.z.boolean().nullable().meta({ omitempty: true }).optional(),
  level: CliLevelSchema.nullable().meta({ omitempty: true }).optional(),
  message: JsonValueSchema,
  type: CliErrorTypeSchema
}).describe('A failure or advisory written to stdout. `fatal: true` means the\nprocess is exiting with a non-zero status; `fatal: false` is a\nnon-blocking warning (e.g. auto-update failed but the requested\ncommand still ran).\n\n`message` is an arbitrary JSON value so producers can emit\nstructured payloads (e.g. `{"code": ..., "detail": ...}`). Wrap\na plain string as `Value::String(...)` (or use `.into()`) and the\nwire bytes stay identical to the old `String`-only shape.\n\nThe `type` field is a single-variant `ErrorType` enum that\nalways serializes to `"error"`. This is what disambiguates the\nuntagged `Output` enum from a notification \u2014 `Output` tries\n`Error` first, and the constant `type:"error"` tag is what\nrejects every non-error wire shape.').meta({ title: "cli.Error" });

// src/viewer/command/agents/get.ts
async function agentsGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/get" }), zod.z.union([CliErrorSchema, AgentGetAgentResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents get: cli produced no output before the end marker");
  }
  return first;
}
async function agentsGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents get: cli produced no output before the end marker");
  }
  return first;
}
async function agentsGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandAgentsInstancesListResponseItemSchema = zod.z.object({
  agent_instance_hierarchy: zod.z.string().describe("Full hierarchy of this agent instance."),
  logged: zod.z.number().int().min(0).max(18446744073709552e3).describe("Total `logs.messages` rows for this agent over all time."),
  queued: zod.z.number().int().min(0).max(18446744073709552e3).describe("Active `message_queue` rows targeting this agent \u2014 counting\nboth direct-AIH rows and rows whose tag is bound to this AIH."),
  tags: zod.z.array(zod.z.string()).describe("Tag names currently bound to this AIH, newest-bound first."),
  timestamp_active: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3).nullable().describe("Timestamp of the most recent `logs.messages` row for this\nagent. `None` when the agent has no logs yet (queue-only).").meta({ omitempty: true }).optional(),
  timestamp_spawned: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3).nullable().describe("Timestamp of the first `logs.messages` row for this agent.\n`None` when the agent has no logs yet (queue-only).").meta({ omitempty: true }).optional()
}).describe("One discovered agent instance under a target. Aggregated from the\n`logs.messages`, `message_queue`, and `tags` tiers.").meta({ title: "cli.command.agents.instances.list.ResponseItem" });

// src/viewer/command/agents/instances/get.ts
function agentsInstancesGetExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/instances/get" }), zod.z.union([CliErrorSchema, CliCommandAgentsInstancesListResponseItemSchema]));
}
function agentsInstancesGetExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/instances/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function agentsInstancesGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/instances/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents instances get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsInstancesGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/instances/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents instances get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsInstancesGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/instances/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents instances get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsInstancesGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/instances/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents instances get response_schema: cli produced no output before the end marker");
  }
  return first;
}
function agentsInstancesListExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/instances/list" }), zod.z.union([CliErrorSchema, CliCommandAgentsInstancesListResponseItemSchema]));
}
function agentsInstancesListExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/instances/list" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function agentsInstancesListRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/instances/list/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents instances list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsInstancesListRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/instances/list/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents instances list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsInstancesListResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/instances/list/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents instances list response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsInstancesListResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/instances/list/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents instances list response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandAgentsListResponseFavoriteSchema = zod.z.union([zod.z.object({
  commit: zod.z.string().nullable().optional(),
  name: zod.z.string(),
  note: zod.z.string(),
  owner: zod.z.string(),
  remote: zod.z.literal("github"),
  repository: zod.z.string()
}).meta({ "variantTitle": "Github" }), zod.z.object({
  commit: zod.z.string().nullable().optional(),
  name: zod.z.string(),
  note: zod.z.string(),
  owner: zod.z.string(),
  remote: zod.z.literal("filesystem"),
  repository: zod.z.string()
}).meta({ "variantTitle": "Filesystem" }), zod.z.object({
  name: zod.z.string(),
  note: zod.z.string(),
  remote: zod.z.literal("mock")
}).meta({ "variantTitle": "Mock" })]).meta({ title: "cli.command.agents.list.ResponseFavorite" });

// src/cli/command/agents/list/responseItem.ts
var CliCommandAgentsListResponseItemSchema = zod.z.union([CliCommandAgentsListResponseFavoriteSchema.meta({ "title": "cli.command.agents.list.ResponseFavorite", "variantTitle": "Favorite" }), RemotePathSchema.meta({ "title": "RemotePath", "variantTitle": "Item" })]).meta({ title: "cli.command.agents.list.ResponseItem" });

// src/viewer/command/agents/list.ts
function agentsListExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/list" }), zod.z.union([CliErrorSchema, CliCommandAgentsListResponseItemSchema]));
}
function agentsListExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/list" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function agentsListRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/list/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsListRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/list/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsListResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/list/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents list response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsListResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/list/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents list response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandAgentsLogsReadAllAssistantResponsePartTypeSchema = zod.z.enum(["refusal", "reasoning", "tool_call", "text", "image", "audio", "video", "file"]).describe("Type tag for one `AssistantResponse` part \u2014 the table-kind of\nthe underlying `assistant_response_*` row.").meta({ title: "cli.command.agents.logs.read.all.AssistantResponsePartType" });

// src/cli/command/agents/logs/read/all/assistantResponsePart.ts
var CliCommandAgentsLogsReadAllAssistantResponsePartSchema = zod.z.object({
  function_name: zod.z.string().describe("`function.name` for `type = tool_call` rows\n(`logs.assistant_response_tool_calls.function_name`).\nEmpty string for non-tool-call rows. Surfaced here so\ncallers can dedupe tool calls by name without a per-row\n`agents logs read id` round-trip."),
  id: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3).describe('`logs.messages."index"` for this row. Pass to\n`agents logs read id <n>` for the typed body.'),
  timestamp_delivered: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  type: CliCommandAgentsLogsReadAllAssistantResponsePartTypeSchema
}).describe("One row inside an `AssistantResponse` block.").meta({ title: "cli.command.agents.logs.read.all.AssistantResponsePart" });
var CliCommandAgentsLogsReadAllClientNotificationPartTypeSchema = zod.z.enum(["text", "image", "audio", "video", "file"]).describe("Type tag for one `ClientNotification` part \u2014 the table-kind of\nthe underlying `message_queue_*` content row.").meta({ title: "cli.command.agents.logs.read.all.ClientNotificationPartType" });

// src/cli/command/agents/logs/read/all/clientNotificationPart.ts
var CliCommandAgentsLogsReadAllClientNotificationPartSchema = zod.z.object({
  id: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3).describe('`logs.messages."index"` for this row. Pass to\n`agents logs read id <n>` to fetch the consumed\n`message_queue_contents` body.'),
  timestamp_delivered: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3).describe('`logs.messages."timestamp"` \u2014 when the receiver consumed\nthis content row and the LogWriter committed the\nconsumption event.'),
  type: CliCommandAgentsLogsReadAllClientNotificationPartTypeSchema
}).describe("One row inside a `ClientNotification` block \u2014 a consumed\n`message_queue_contents` entry. `timestamp_queued` is on the\nenclosing block (it lives on `message_queue.enqueued_at`, not\nper-content); only the per-row consumption timestamp is here.").meta({ title: "cli.command.agents.logs.read.all.ClientNotificationPart" });
var CliCommandAgentsLogsReadAllToolResponsePartTypeSchema = zod.z.enum(["container", "text", "image", "audio", "video", "file"]).describe("Type tag for one `ToolResponse` part. `Container` is the\n`tool_response` head row (carries `tool_call_id`); the other\nfive are content slots.").meta({ title: "cli.command.agents.logs.read.all.ToolResponsePartType" });

// src/cli/command/agents/logs/read/all/toolResponsePart.ts
var CliCommandAgentsLogsReadAllToolResponsePartSchema = zod.z.object({
  id: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3).describe('`logs.messages."index"` for this row. Pass to\n`agents logs read id <n>` for the typed body.'),
  timestamp_delivered: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  type: CliCommandAgentsLogsReadAllToolResponsePartTypeSchema
}).describe("One row inside a `ToolResponse` block.").meta({ title: "cli.command.agents.logs.read.all.ToolResponsePart" });

// src/cli/command/agents/logs/read/all/responseItem.ts
var CliCommandAgentsLogsReadAllResponseItemSchema = zod.z.union([zod.z.object({
  agent_instance_hierarchy: zod.z.string(),
  id: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  response_id: zod.z.string(),
  sender_agent_instance_hierarchy: zod.z.string().describe("AIH of the caller who issued the request \u2014 from\n`logs.agent_completion_requests.sender_*`."),
  timestamp_delivered: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  type: zod.z.literal("agent_completion_request")
}).meta({ "variantTitle": "AgentCompletionRequest" }), zod.z.object({
  agent_instance_hierarchy: zod.z.string(),
  id: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  response_id: zod.z.string(),
  sender_agent_instance_hierarchy: zod.z.string(),
  timestamp_delivered: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  type: zod.z.literal("vector_completion_request")
}).meta({ "variantTitle": "VectorCompletionRequest" }), zod.z.object({
  agent_instance_hierarchy: zod.z.string(),
  id: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  response_id: zod.z.string(),
  sender_agent_instance_hierarchy: zod.z.string(),
  timestamp_delivered: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  type: zod.z.literal("function_execution_request")
}).meta({ "variantTitle": "FunctionExecutionRequest" }), zod.z.object({
  agent_instance_hierarchy: zod.z.string(),
  key: zod.z.string().nullable().describe("Idempotency token, if the row was enqueued with\n`--key` via `agents message --enqueue-with-key`.\nSurfacing it lets readers attribute a notification\nto a specific enqueue beyond just the sender AIH.").meta({ omitempty: true }).optional(),
  parts: zod.z.array(CliCommandAgentsLogsReadAllClientNotificationPartSchema),
  response_id: zod.z.string(),
  sender_agent_instance_hierarchy: zod.z.string().describe("AIH of the enqueuer \u2014 from `message_queue.sender_*`\njoined through `message_queue_contents.id`."),
  timestamp_queued: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3).describe("`message_queue.enqueued_at` of the consumed parent\nqueue row. One block = one parent queue row, so this\nis well-defined block-level (each part's individual\n`timestamp_delivered` still records its own\nconsumption moment)."),
  type: zod.z.literal("client_notification")
}).meta({ "variantTitle": "ClientNotification" }), zod.z.object({
  agent_instance_hierarchy: zod.z.string(),
  parts: zod.z.array(CliCommandAgentsLogsReadAllAssistantResponsePartSchema),
  response_id: zod.z.string(),
  type: zod.z.literal("assistant_response")
}).describe("Agent emissions \u2014 the agent IS the producer of these\nrows, so there's no separate sender. The\n`agent_instance_hierarchy` field IS the sender.").meta({ "variantTitle": "AssistantResponse" }), zod.z.object({
  agent_instance_hierarchy: zod.z.string(),
  parts: zod.z.array(CliCommandAgentsLogsReadAllToolResponsePartSchema),
  response_id: zod.z.string(),
  type: zod.z.literal("tool_response")
}).meta({ "variantTitle": "ToolResponse" })]).describe("One yielded item. Three single-row request blobs +\nthree multi-row blocks. Every variant carries `response_id`.\n`sender_agent_instance_hierarchy` appears only on the four\nvariants that have a sender \u2260 producer: the three request\nvariants (caller AIH) and `ClientNotification` (enqueuer\nAIH). `AssistantResponse` and `ToolResponse` are emitted BY\nthe agent itself \u2014 their `agent_instance_hierarchy` IS the\nproducer, so no separate sender field exists.\n\nBlock-coalescing boundary tuple: `(class,\nagent_instance_hierarchy, response_id)` for assistant/tool\nblocks; `(class, agent_instance_hierarchy, response_id,\nsender, message_queue_id)` for `ClientNotification` blocks.\nOne `ClientNotification` block = one consumed\n`message_queue` parent row, so `timestamp_queued` and\n`sender_agent_instance_hierarchy` are well-defined\nblock-level.").meta({ title: "cli.command.agents.logs.read.all.ResponseItem" });

// src/viewer/command/agents/logs/read/all.ts
function agentsLogsReadAllExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/logs/read/all" }), zod.z.union([CliErrorSchema, CliCommandAgentsLogsReadAllResponseItemSchema]));
}
function agentsLogsReadAllExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/logs/read/all" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function agentsLogsReadAllRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/logs/read/all/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read all request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsLogsReadAllRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/logs/read/all/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read all request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsLogsReadAllResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/logs/read/all/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read all response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsLogsReadAllResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/logs/read/all/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read all response_schema: cli produced no output before the end marker");
  }
  return first;
}
var AgentCompletionsMessageFileSchema = zod.z.object({
  file_data: zod.z.string().nullable().describe("Base64-encoded file data.").meta({ omitempty: true }).optional(),
  file_id: zod.z.string().nullable().describe("The ID of a previously uploaded file.").meta({ omitempty: true }).optional(),
  file_url: zod.z.string().nullable().describe("A URL to fetch the file from.").meta({ omitempty: true }).optional(),
  filename: zod.z.string().nullable().describe("The filename for display purposes.").meta({ omitempty: true }).optional()
}).describe("A file attachment for multimodal input.").meta({ title: "agent.completions.message.File" });
var AgentCompletionsMessageImageUrlDetailSchema = zod.z.union([zod.z.literal("auto").describe("Let the model decide the detail level.").meta({ "variantTitle": "Auto" }), zod.z.literal("low").describe("Low detail mode (faster, less tokens).").meta({ "variantTitle": "Low" }), zod.z.literal("high").describe("High detail mode (more accurate, more tokens).").meta({ "variantTitle": "High" })]).describe("Detail level for image processing.").meta({ title: "agent.completions.message.ImageUrlDetail" });

// src/agent/completions/message/imageUrl.ts
var AgentCompletionsMessageImageUrlSchema = zod.z.object({
  detail: AgentCompletionsMessageImageUrlDetailSchema.nullable().describe("The detail level for image processing.").meta({ omitempty: true }).optional(),
  url: zod.z.string().describe("The URL of the image (can be a data URL or HTTP URL).")
}).describe("An image URL for multimodal input.").meta({ title: "agent.completions.message.ImageUrl" });
var AgentCompletionsMessageInputAudioSchema = zod.z.object({
  data: zod.z.string().describe("Base64-encoded audio data."),
  format: zod.z.string().describe('The audio format (e.g., "wav", "mp3").')
}).describe("Audio input for multimodal messages.").meta({ title: "agent.completions.message.InputAudio" });
var AgentCompletionsMessageVideoUrlSchema = zod.z.object({
  url: zod.z.string().describe("The URL of the video.")
}).describe("A video URL for multimodal input.").meta({ title: "agent.completions.message.VideoUrl" });
var AgentCompletionsMessageAssistantToolCallFunctionSchema = zod.z.object({
  arguments: zod.z.string().describe("The arguments to pass to the function, as a JSON string."),
  name: zod.z.string().describe("The name of the function to call.")
}).describe("Details of a function call made by the assistant.").meta({ title: "agent.completions.message.AssistantToolCallFunction" });

// src/agent/completions/message/assistantToolCall.ts
var AgentCompletionsMessageAssistantToolCallSchema = zod.z.object({
  function: AgentCompletionsMessageAssistantToolCallFunctionSchema.describe("The function being called."),
  id: zod.z.string().describe("The unique ID of this tool call."),
  type: zod.z.literal("function")
}).describe("A function call with an ID and function details.").meta({ title: "agent.completions.message.AssistantToolCall" });
var AgentCompletionsMessageRichContentPartSchema = zod.z.union([zod.z.object({
  text: zod.z.string(),
  type: zod.z.literal("text")
}).describe("Text content.").meta({ "variantTitle": "Text" }), zod.z.object({
  image_url: AgentCompletionsMessageImageUrlSchema,
  type: zod.z.literal("image_url")
}).describe("An image URL.").meta({ "variantTitle": "ImageUrl" }), zod.z.object({
  input_audio: AgentCompletionsMessageInputAudioSchema,
  type: zod.z.literal("input_audio")
}).describe("Audio input.").meta({ "variantTitle": "InputAudio" }), zod.z.object({
  type: zod.z.literal("input_video"),
  video_url: AgentCompletionsMessageVideoUrlSchema
}).describe("Video input.").meta({ "variantTitle": "InputVideo" }), zod.z.object({
  type: zod.z.literal("video_url"),
  video_url: AgentCompletionsMessageVideoUrlSchema
}).describe("A video URL.").meta({ "variantTitle": "VideoUrl" }), zod.z.object({
  file: AgentCompletionsMessageFileSchema,
  type: zod.z.literal("file")
}).describe("A file.").meta({ "variantTitle": "File" })]).describe("A part of rich content.").meta({ title: "agent.completions.message.RichContentPart" });

// src/agent/completions/message/richContent.ts
var AgentCompletionsMessageRichContentSchema = zod.z.union([zod.z.string().describe("Plain text content.").meta({ "variantTitle": "Text" }), zod.z.array(AgentCompletionsMessageRichContentPartSchema).describe("Multi-part content (text, images, audio, video, files).").meta({ "variantTitle": "Parts" })]).describe("Rich content for user/assistant messages (supports multimodal input).").meta({ title: "agent.completions.message.RichContent" });

// src/agent/completions/message/assistantMessage.ts
var AgentCompletionsMessageAssistantMessageSchema = zod.z.object({
  content: AgentCompletionsMessageRichContentSchema.nullable().describe("The message content, if any.").meta({ omitempty: true }).optional(),
  name: zod.z.string().nullable().describe("Optional name for the assistant.").meta({ omitempty: true }).optional(),
  reasoning: zod.z.string().nullable().describe("Reasoning content from models that support chain-of-thought.").meta({ omitempty: true }).optional(),
  refusal: zod.z.string().nullable().describe("Refusal message if the model declined to respond.").meta({ omitempty: true }).optional(),
  tool_calls: zod.z.array(AgentCompletionsMessageAssistantToolCallSchema).nullable().describe("Tool calls made by the assistant.").meta({ omitempty: true }).optional()
}).describe("An assistant message (model's previous response).").meta({ title: "agent.completions.message.AssistantMessage" });
var AgentCompletionsMessageSimpleContentPartSchema = zod.z.object({
  text: zod.z.string().describe("The text content."),
  type: zod.z.literal("text")
}).describe("A text part.").meta({ title: "agent.completions.message.SimpleContentPart" });

// src/agent/completions/message/simpleContent.ts
var AgentCompletionsMessageSimpleContentSchema = zod.z.union([zod.z.string().describe("Plain text content.").meta({ "variantTitle": "Text" }), zod.z.array(AgentCompletionsMessageSimpleContentPartSchema).describe("Multi-part text content.").meta({ "variantTitle": "Parts" })]).describe("Simple text content for system/developer messages.").meta({ title: "agent.completions.message.SimpleContent" });

// src/agent/completions/message/developerMessage.ts
var AgentCompletionsMessageDeveloperMessageSchema = zod.z.object({
  content: AgentCompletionsMessageSimpleContentSchema.describe("The message content."),
  name: zod.z.string().nullable().describe("Optional name for the message author.").meta({ omitempty: true }).optional()
}).describe("A developer message.").meta({ title: "agent.completions.message.DeveloperMessage" });
var AgentCompletionsMessageSystemMessageSchema = zod.z.object({
  content: AgentCompletionsMessageSimpleContentSchema.describe("The message content."),
  name: zod.z.string().nullable().describe("Optional name for the message author.").meta({ omitempty: true }).optional()
}).describe("A system message setting context or instructions.").meta({ title: "agent.completions.message.SystemMessage" });
var AgentCompletionsMessageToolResponseMetadataSchema = zod.z.object({
  notifications: zod.z.number().int().min(0).max(18446744073709552e3).nullable().describe("Count of pending notifications the proxy drained and prepended\nto the tool response's `content` before returning. Only set\nwhen at least one notification was drained.").meta({ omitempty: true }).optional()
}).describe("Vendor-extension metadata attached to a tool response. The\n`objectiveai-mcp-proxy` populates known keys (currently\n`notifications`); the SDK lossy-decodes the MCP `_meta` bag into\nthis typed shape. Unknown keys are dropped.").meta({ title: "agent.completions.message.ToolResponseMetadata" });

// src/agent/completions/message/toolMessage.ts
var AgentCompletionsMessageToolMessageSchema = zod.z.object({
  content: AgentCompletionsMessageRichContentSchema.describe("The content of the tool response."),
  metadata: AgentCompletionsMessageToolResponseMetadataSchema.nullable().describe("Optional vendor-extension metadata, populated by\n`objectiveai-mcp-proxy` via MCP's `_meta` extension bag.").meta({ omitempty: true }).optional(),
  tool_call_id: zod.z.string().describe("The ID of the tool call this message responds to.")
}).describe("A tool message containing the result of a tool call.").meta({ title: "agent.completions.message.ToolMessage" });
var AgentCompletionsMessageUserMessageSchema = zod.z.object({
  content: AgentCompletionsMessageRichContentSchema.describe("The message content (supports text, images, audio, video, files)."),
  name: zod.z.string().nullable().describe("Optional name for the user.").meta({ omitempty: true }).optional()
}).describe("A user message from the end user.").meta({ title: "agent.completions.message.UserMessage" });

// src/agent/completions/message/message.ts
var AgentCompletionsMessageMessageSchema = zod.z.union([AgentCompletionsMessageDeveloperMessageSchema.and(zod.z.object({
  role: zod.z.literal("developer")
})).describe("A developer message (similar to system, but from the developer).").meta({ "variantTitle": "Developer" }), AgentCompletionsMessageSystemMessageSchema.and(zod.z.object({
  role: zod.z.literal("system")
})).describe("A system message setting context or instructions.").meta({ "variantTitle": "System" }), AgentCompletionsMessageUserMessageSchema.and(zod.z.object({
  role: zod.z.literal("user")
})).describe("A user message from the end user.").meta({ "variantTitle": "User" }), AgentCompletionsMessageAssistantMessageSchema.and(zod.z.object({
  role: zod.z.literal("assistant")
})).describe("An assistant message (model's previous response).").meta({ "variantTitle": "Assistant" }), AgentCompletionsMessageToolMessageSchema.and(zod.z.object({
  role: zod.z.literal("tool")
})).describe("A tool message containing the result of a tool call.").meta({ "variantTitle": "Tool" })]).describe("A message in the conversation.").meta({ title: "agent.completions.message.Message" });
var AgentCompletionsRequestProviderDataCollectionSchema = zod.z.union([zod.z.literal("deny").describe("Do not allow data collection.").meta({ "variantTitle": "Deny" }), zod.z.literal("allow").describe("Allow data collection.").meta({ "variantTitle": "Allow" })]).describe("Data collection policy for providers.").meta({ title: "agent.completions.request.ProviderDataCollection" });
var AgentCompletionsRequestProviderMaxPriceSchema = zod.z.object({
  audio: zod.z.number().min(-34028234663852886e22).max(34028234663852886e22).nullable().describe("Maximum price per audio second.").meta({ omitempty: true }).optional(),
  completion: zod.z.number().min(-34028234663852886e22).max(34028234663852886e22).nullable().describe("Maximum price per completion token.").meta({ omitempty: true }).optional(),
  image: zod.z.number().min(-34028234663852886e22).max(34028234663852886e22).nullable().describe("Maximum price per image.").meta({ omitempty: true }).optional(),
  prompt: zod.z.number().min(-34028234663852886e22).max(34028234663852886e22).nullable().describe("Maximum price per prompt token.").meta({ omitempty: true }).optional(),
  request: zod.z.number().min(-34028234663852886e22).max(34028234663852886e22).nullable().describe("Maximum price per request.").meta({ omitempty: true }).optional()
}).describe("Maximum price constraints per token type.").meta({ title: "agent.completions.request.ProviderMaxPrice" });
var AgentCompletionsRequestProviderSortSchema = zod.z.union([zod.z.literal("price").describe("Prioritize by price (cheapest first).").meta({ "variantTitle": "Price" }), zod.z.literal("throughput").describe("Prioritize by throughput (fastest first).").meta({ "variantTitle": "Throughput" }), zod.z.literal("latency").describe("Prioritize by latency (lowest first).").meta({ "variantTitle": "Latency" })]).describe("How to sort/prioritize providers.").meta({ title: "agent.completions.request.ProviderSort" });

// src/agent/completions/request/provider.ts
var AgentCompletionsRequestProviderSchema = zod.z.object({
  data_collection: AgentCompletionsRequestProviderDataCollectionSchema.nullable().describe("Whether to allow providers to collect data.").meta({ omitempty: true }).optional(),
  max_latency: zod.z.number().min(-34028234663852886e22).max(34028234663852886e22).nullable().describe("Hard maximum latency requirement (seconds).").meta({ omitempty: true }).optional(),
  max_price: AgentCompletionsRequestProviderMaxPriceSchema.nullable().describe("Maximum price constraints.").meta({ omitempty: true }).optional(),
  min_throughput: zod.z.number().min(-34028234663852886e22).max(34028234663852886e22).nullable().describe("Hard minimum throughput requirement (tokens/second).").meta({ omitempty: true }).optional(),
  preferred_max_latency: zod.z.number().min(-34028234663852886e22).max(34028234663852886e22).nullable().describe("Preferred maximum latency (seconds).").meta({ omitempty: true }).optional(),
  preferred_min_throughput: zod.z.number().min(-34028234663852886e22).max(34028234663852886e22).nullable().describe("Preferred minimum throughput (tokens/second).").meta({ omitempty: true }).optional(),
  sort: AgentCompletionsRequestProviderSortSchema.nullable().describe("How to sort/prioritize providers.").meta({ omitempty: true }).optional(),
  zdr: zod.z.boolean().nullable().describe("Whether to use zero data retention providers only.").meta({ omitempty: true }).optional()
}).describe("Provider routing and selection preferences.").meta({ title: "agent.completions.request.Provider" });
var AgentCompletionsRequestResponseFormatSchema = zod.z.union([zod.z.object({
  type: zod.z.literal("text")
}).describe("Plain text response (default).").meta({ "variantTitle": "Text" }), zod.z.object({
  type: zod.z.literal("json_object")
}).describe("Response must be valid JSON.").meta({ "variantTitle": "JsonObject" }), zod.z.object({
  schema: zod.z.record(zod.z.string(), JsonValueSchema).describe("The JSON Schema definition."),
  type: zod.z.literal("json_schema")
}).describe("Response must conform to a JSON schema.").meta({ "variantTitle": "JsonSchema" }), zod.z.object({
  grammar: zod.z.string(),
  type: zod.z.literal("grammar")
}).describe("Response must conform to a grammar.").meta({ "variantTitle": "Grammar" }), zod.z.object({
  type: zod.z.literal("python")
}).describe("Response must be valid Python code.").meta({ "variantTitle": "Python" }), zod.z.object({
  description: zod.z.string().describe("A description of the tool."),
  name: zod.z.string().describe("The name of the tool."),
  required: zod.z.boolean().nullable().describe("Whether the tool MUST be called.").meta({ omitempty: true }).optional(),
  schema: zod.z.record(zod.z.string(), JsonValueSchema).describe("The JSON Schema definition."),
  type: zod.z.literal("tool_call")
}).describe("The final assistant message will contain this tool call").meta({ "variantTitle": "ToolCall" })]).describe("The format of the model's response.").meta({ title: "agent.completions.request.ResponseFormat" });

// src/agent/completions/request/responseFormatParam.ts
var AgentCompletionsRequestResponseFormatParamSchema = zod.z.union([AgentCompletionsRequestResponseFormatSchema.describe("A single response format applied to all agents.").meta({ "title": "agent.completions.request.ResponseFormat", "variantTitle": "Single" }), zod.z.record(zod.z.string(), AgentCompletionsRequestResponseFormatSchema).describe("Per-agent response formats, keyed by agent ID.").meta({ "variantTitle": "PerAgent" })]).describe("Either a single response format or a per-agent map.").meta({ title: "agent.completions.request.ResponseFormatParam" });
var AgentClaudeAgentSdkEffortSchema = zod.z.union([zod.z.literal("low").describe("Minimal output, concise responses.").meta({ "variantTitle": "Low" }), zod.z.literal("medium").describe("Balanced output (default, normalized away during preparation).").meta({ "variantTitle": "Medium" }), zod.z.literal("high").describe("Detailed output with thorough explanations.").meta({ "variantTitle": "High" }), zod.z.literal("xhigh").describe("Extra-high effort, above `High` but below `Max`.").meta({ "variantTitle": "Xhigh" }), zod.z.literal("max").describe("Maximum effort, most detailed output possible.").meta({ "variantTitle": "Max" })]).describe("The effort level for model output.\n\nThis setting hints to the model how detailed its responses should be.").meta({ title: "agent.claude_agent_sdk.Effort" });
var AgentClaudeAgentSdkOutputModeSchema = zod.z.literal("instruction").describe("The model is instructed via the prompt to output a specific key.\n\nThis is the default and most widely supported mode.").meta({ title: "agent.claude_agent_sdk.OutputMode" });
var AgentClaudeAgentSdkUpstreamSchema = zod.z.literal("claude_agent_sdk").describe("Claude Agent SDK upstream marker.").meta({ title: "agent.claude_agent_sdk.Upstream" });
var AgentClientObjectiveaiMcpEntrySchema = zod.z.object({
  name: zod.z.string(),
  owner: zod.z.string(),
  version: zod.z.string()
}).describe("A single `owner` / `name` / `version` reference identifying one\ntool inside [`ClientObjectiveaiMcp::tools`]. Plugin references\nuse the larger [`ClientObjectiveaiMcpPluginEntry`] \u2014 they carry\nextra `executable` / `mcp_servers` fields tools don't have.").meta({ title: "agent.ClientObjectiveaiMcpEntry" });
var AgentClientObjectiveaiMcpPluginMcpServerSchema = zod.z.object({
  arguments: zod.z.record(zod.z.string(), zod.z.string().nullable()).nullable().describe('Optional key\u2192value arguments forwarded to the plugin alongside\n`name`. `Some(value)` \u21D2 `--key value` on the spawned plugin\'s\nargv; `None` \u21D2 a bare `--key` flag with no following token. The\nplugin author decides how to interpret them. [`prepare`]\nnormalizes (`Some("") \u2192 None`), sorts the map by key, and\ncollapses an empty map to `None` so two equivalent declarations\ncanonicalize to byte-identical JSON.').meta({ omitempty: true }).optional(),
  name: zod.z.string().describe("Author-chosen identifier \u2014 must match a `name` in the plugin\nmanifest's `mcp_servers` list. The CLI feeds this as the\nfirst positional arg when starting the plugin.")
}).describe("One `mcp_servers` entry on a\n[`ClientObjectiveaiMcpPluginEntry`]: the manifest-declared `name`\nthe agent wants exposed, plus optional `arguments` the CLI feeds\nto the plugin alongside the name when bringing the server up. The\narguments map is sorted by key in [`prepare`] so two equivalent\ndeclarations (same key/value pairs in any order) hash to the same\ncanonical form.").meta({ title: "agent.ClientObjectiveaiMcpPluginMcpServer" });

// src/agent/clientObjectiveaiMcpPluginEntry.ts
var AgentClientObjectiveaiMcpPluginEntrySchema = zod.z.object({
  executable: zod.z.boolean().default(true).describe("`true`: spawn the plugin binary and surface its tools the\nusual way. `false`: don't run the plugin; only consume its\ndeclared MCP servers (`mcp_servers` below). Defaults to\n`true` so existing declarations keep their current behavior."),
  mcp_servers: zod.z.array(AgentClientObjectiveaiMcpPluginMcpServerSchema).nullable().describe("Subset of the plugin's manifest `mcp_servers` to expose, each\nreferenced by `name` plus an optional `arguments` map. `None`\n\u21D2 none. Names that aren't present in the plugin's manifest are\nrejected when the API asks the CLI to begin them; declarations\nthemselves don't validate the referent (the plugin may not be\ninstalled at declaration time).").meta({ omitempty: true }).optional(),
  name: zod.z.string(),
  owner: zod.z.string(),
  version: zod.z.string()
}).describe("Plugin reference inside [`ClientObjectiveaiMcp::plugins`].\n\n- `owner` / `name` / `version` identify the plugin (same shape as\n  [`ClientObjectiveaiMcpEntry`]).\n- `executable` controls whether this plugin contributes a tool\n  to the agent's surface (`true`, default) or is loaded purely\n  for its declared MCP servers (`false`). The API's\n  `X-OBJECTIVEAI-TOOLS-ALLOWED` construction honors this: only\n  `executable = true` plugin entries contribute their `name` to\n  the allow-list.\n- `mcp_servers` selects which of the plugin's manifest-declared\n  `filesystem::plugins::Manifest::mcp_servers` entries should be\n  exposed to the agent, by `name`. `None` \u21D2 none of them.").meta({ title: "agent.ClientObjectiveaiMcpPluginEntry" });

// src/agent/clientObjectiveaiMcp.ts
var AgentClientObjectiveaiMcpSchema = zod.z.object({
  objectiveai: zod.z.boolean().nullable().meta({ omitempty: true }).optional(),
  plugins: zod.z.array(AgentClientObjectiveaiMcpPluginEntrySchema).meta({ omitempty: true }),
  tools: zod.z.array(AgentClientObjectiveaiMcpEntrySchema).meta({ omitempty: true })
}).describe("Client-side MCP surface the agent expects:\n\n- `objectiveai`: whether the calling client exposes the built-in\n  `objectiveai-mcp`. `None` means unspecified; `Some(true)` /\n  `Some(false)` explicitly opt in / out.\n- `plugins`: specific plugins (by `owner` / `name` / `version`)\n  plus per-plugin `executable` + `mcp_servers`.\n- `tools`: specific tools (by `owner` / `name` / `version`).").meta({ title: "agent.ClientObjectiveaiMcp" });
var AgentMcpServerSchema = zod.z.object({
  authorization: zod.z.boolean().default(false).describe("Whether this MCP server uses authorization."),
  url: zod.z.string().describe("The URL of the MCP server.")
}).describe("An MCP server that the agent can connect to.").meta({ title: "agent.McpServer" });

// src/agent/claude_agent_sdk/agentBase.ts
var AgentClaudeAgentSdkAgentBaseSchema = zod.z.object({
  client_objectiveai_mcp: AgentClientObjectiveaiMcpSchema.nullable().describe("Client-side ObjectiveAI MCP surface the calling client is\nexpected to expose locally back to the API (objectiveai\nbuilt-in, plus specific plugins / tools by owner+name+version).").meta({ omitempty: true }).optional(),
  effort: AgentClaudeAgentSdkEffortSchema.nullable().describe("The effort level for model output.").meta({ omitempty: true }).optional(),
  mcp_servers: zod.z.array(AgentMcpServerSchema).nullable().describe("MCP servers the agent can connect to.").meta({ omitempty: true }).optional(),
  model: zod.z.string().describe("The upstream language model identifier."),
  output_mode: AgentClaudeAgentSdkOutputModeSchema.describe("The output mode for vector completions. Ignored for agent completions."),
  prefix_content: AgentCompletionsMessageRichContentSchema.nullable().describe("Rich content prepended to the user's prompt.").meta({ omitempty: true }).optional(),
  suffix_content: AgentCompletionsMessageRichContentSchema.nullable().describe("Rich content appended after the user's prompt.").meta({ omitempty: true }).optional(),
  system_prompt: zod.z.string().nullable().describe("System prompt for the agent.").meta({ omitempty: true }).optional(),
  thinking: zod.z.boolean().nullable().describe("Whether thinking/extended thinking is enabled.\n\nDefaults to `true`. Set to `false` to disable.").meta({ omitempty: true }).optional(),
  upstream: AgentClaudeAgentSdkUpstreamSchema.describe("The upstream provider marker.")
}).describe("The base configuration for a Claude Agent SDK Agent (without computed ID).").meta({ title: "agent.claude_agent_sdk.AgentBase" });
var AgentCodexSdkEffortSchema = zod.z.union([zod.z.literal("minimal").describe("Minimal reasoning, fastest responses.").meta({ "variantTitle": "Minimal" }), zod.z.literal("low").describe("Low reasoning effort.").meta({ "variantTitle": "Low" }), zod.z.literal("medium").describe("Balanced reasoning (default, normalized away during preparation).").meta({ "variantTitle": "Medium" }), zod.z.literal("high").describe("High reasoning effort.").meta({ "variantTitle": "High" })]).describe("Reasoning-effort level for the Codex model. Maps to Codex's\n`model_reasoning_effort` (see `openai_codex_sdk` `ModelReasoningEffort`).\n\n`Medium` is the default and is normalized to `None` during preparation\nfor content-addressing stability.").meta({ title: "agent.codex_sdk.Effort" });
var AgentCodexSdkOutputModeSchema = zod.z.literal("instruction").describe("The model is instructed via the prompt to output a specific key.\n\nThis is the default and most widely supported mode.").meta({ title: "agent.codex_sdk.OutputMode" });
var AgentCodexSdkUpstreamSchema = zod.z.literal("codex_sdk").describe("Codex SDK upstream marker.").meta({ title: "agent.codex_sdk.Upstream" });

// src/agent/codex_sdk/agentBase.ts
var AgentCodexSdkAgentBaseSchema = zod.z.object({
  client_objectiveai_mcp: AgentClientObjectiveaiMcpSchema.nullable().describe("Client-side ObjectiveAI MCP surface the calling client is\nexpected to expose locally back to the API (objectiveai\nbuilt-in, plus specific plugins / tools by owner+name+version).").meta({ omitempty: true }).optional(),
  effort: AgentCodexSdkEffortSchema.nullable().describe("Reasoning effort \u2014 maps to Codex's `model_reasoning_effort`.").meta({ omitempty: true }).optional(),
  mcp_servers: zod.z.array(AgentMcpServerSchema).nullable().describe("MCP servers the agent can connect to.").meta({ omitempty: true }).optional(),
  model: zod.z.string().describe("The upstream language model identifier (e.g. `gpt-5`)."),
  output_mode: AgentCodexSdkOutputModeSchema.describe("The output mode for vector completions. Ignored for agent completions."),
  prefix_content: AgentCompletionsMessageRichContentSchema.nullable().describe("Rich content prepended to the user's prompt.").meta({ omitempty: true }).optional(),
  suffix_content: AgentCompletionsMessageRichContentSchema.nullable().describe("Rich content appended after the user's prompt.").meta({ omitempty: true }).optional(),
  upstream: AgentCodexSdkUpstreamSchema.describe("The upstream provider marker."),
  web_search_enabled: zod.z.boolean().nullable().describe("Whether this agent may use the codex binary's web-search tool.").meta({ omitempty: true }).optional()
}).describe("The base configuration for a Codex SDK Agent (without computed ID).").meta({ title: "agent.codex_sdk.AgentBase" });
var AgentMockCallToolCallSchema = zod.z.object({
  arguments: zod.z.string().describe("JSON-string arguments \u2014 same wire shape as\n`AssistantToolCallFunction::arguments`. Passed through to the\nemitted tool call verbatim; validation is the downstream\n`tools/call` handler's job."),
  name: zod.z.string().describe("Tool name. Matched against the upstream tool surface at call\ntime the same way a model-emitted tool call would be.")
}).describe("A single tool call within a [`Call`]. Identifies the tool by\n`name` and carries the exact JSON-encoded arguments to pass. No\n`id` field \u2014 call ids on the wire are minted randomly per emit\nand are ignored by the override's match check.").meta({ title: "agent.mock.CallToolCall" });

// src/agent/mock/call.ts
var AgentMockCallSchema = zod.z.object({
  content: zod.z.string().describe('Assistant text content emitted *after* the tool calls. Plain\n`String` \u2014 the mock\'s chunk-emission path only produces\n`RichContent::Text`, so multimodal content was never possible\nhere anyway. An empty string is treated as "no content" by\nthe emitter (the wire shape becomes `content: None`) and\nmatches an assistant message with `content: None`.'),
  tool_calls: zod.z.array(AgentMockCallToolCallSchema).describe('Tool calls emitted by this turn. Empty `Vec` is allowed and\nmeans "no tool calls, just content."').meta({ omitempty: true })
}).describe("One scripted assistant turn for the mock agent's deterministic\n`calls` override (see [`super::AgentBase::calls`]).\n\nWhen the mock fires this `Call`, it emits every entry in\n`tool_calls` first (as tool-call deltas), then `content` (as\ncontent deltas), finishing the turn after the last content chunk.\nThe match check on the continuation compares\n`(tool_calls[i].name, tool_calls[i].arguments)` pairs plus the\nfull `content` string \u2014 call ids are intentionally excluded since\nthey're random per-emit.").meta({ title: "agent.mock.Call" });
var AgentMockModeSchema = zod.z.enum(["default", "invention", "laboratory_builder", "laboratory_evaluation"]).meta({ title: "agent.mock.Mode" });
var AgentMockOutputModeSchema = zod.z.union([zod.z.literal("instruction").describe("The model is instructed via the prompt to output a specific key.\n\nThis is the default and most widely supported mode.").meta({ "variantTitle": "Instruction" }), zod.z.literal("json_schema").describe("A JSON schema response format is used with an enum of possible keys.\n\nRequires model support for structured JSON output.").meta({ "variantTitle": "JsonSchema" }), zod.z.literal("tool_call").describe("A forced tool call with an argument schema containing possible keys.\n\nRequires model support for tool/function calling.").meta({ "variantTitle": "ToolCall" })]).describe("The method used to constrain LLM output to valid response keys.\n\nIn vector completions, the model must select from a predefined set of\nresponses. This enum controls *how* that constraint is enforced.\n\n**Note:** This setting is only relevant for vector completions and is\ncompletely ignored for agent completions.").meta({ title: "agent.mock.OutputMode" });
var AgentMockUpstreamSchema = zod.z.literal("mock").describe("Mock upstream marker.").meta({ title: "agent.mock.Upstream" });

// src/agent/mock/agentBase.ts
var AgentMockAgentBaseSchema = zod.z.object({
  calls: zod.z.array(AgentMockCallSchema).nullable().describe("Deterministic-script override. When `Some`, the mock agent\nemits each [`super::Call`] as its own assistant turn \u2014\n`tool_calls` first, then `content` \u2014 in array order. Each\nsubsequent turn inspects the continuation to count how many\n`Call`s have already been satisfied (assistant message with\nexactly that `Call`'s `tool_calls` (by name+arguments) and\n`content`); the next un-matched `Call` is what that turn\nemits. Once every `Call` has been satisfied in the\ncontinuation, the mock falls through to its normal mode-driven\ndispatcher. Pure addition \u2014 agents without `calls` are\nunaffected.").meta({ omitempty: true }).optional(),
  client_objectiveai_mcp: AgentClientObjectiveaiMcpSchema.nullable().describe("Client-side ObjectiveAI MCP surface the calling client is\nexpected to expose locally back to the API (objectiveai\nbuilt-in, plus specific plugins / tools by owner+name+version).").meta({ omitempty: true }).optional(),
  error: zod.z.boolean().nullable().describe("If true, the mock client will return an error instead of a response.").meta({ omitempty: true }).optional(),
  error_probability: zod.z.number().int().min(0).max(255).nullable().describe("Probability (0-100) that the mock returns an error mid-stream.\nRequires `error` to be `Some(true)`.").meta({ omitempty: true }).optional(),
  mcp_servers: zod.z.array(AgentMcpServerSchema).nullable().describe("MCP servers the agent can connect to.").meta({ omitempty: true }).optional(),
  mode: AgentMockModeSchema.nullable().describe("Mock agent mode. Defaults to `default`.").meta({ omitempty: true }).optional(),
  output_mode: AgentMockOutputModeSchema.describe("The output mode for vector completions. Ignored for agent completions."),
  top_logprobs: zod.z.number().int().min(0).max(18446744073709552e3).nullable().describe("Number of top log probabilities to return (2-20).\n\n**Vector completions only.** Ignored for agent completions.").meta({ omitempty: true }).optional(),
  upstream: AgentMockUpstreamSchema.describe("The upstream provider marker.")
}).describe("The base configuration for a Mock Agent (without computed ID).").meta({ title: "agent.mock.AgentBase" });
var AgentOpenrouterContextCompressionSchema = zod.z.literal("middle-out").describe("Middle-out compression \u2014 drops content from the middle of the\nconversation when the request would otherwise exceed the\nmodel's context window. The only engine documented today.\n\nIntentionally no `#[schemars(title)]` here: the builder's\nsingle-variant-anyOf flatten merges the variant's title onto\nthe parent schema, clobbering the enum's `agent.openrouter\n.ContextCompression` rename. Add a per-variant title only\nonce a second variant lands.").meta({ title: "agent.openrouter.ContextCompression" });
var AgentOpenrouterOutputModeSchema = zod.z.union([zod.z.literal("instruction").describe("The model is instructed via the prompt to output a specific key.\n\nThis is the default and most widely supported mode.").meta({ "variantTitle": "Instruction" }), zod.z.literal("json_schema").describe("A JSON schema response format is used with an enum of possible keys.\n\nRequires model support for structured JSON output.").meta({ "variantTitle": "JsonSchema" }), zod.z.literal("tool_call").describe("A forced tool call with an argument schema containing possible keys.\n\nRequires model support for tool/function calling.").meta({ "variantTitle": "ToolCall" })]).describe("The method used to constrain LLM output to valid response keys.\n\nIn vector completions, the model must select from a predefined set of\nresponses. This enum controls *how* that constraint is enforced.\n\n**Note:** This setting is only relevant for vector completions and is\ncompletely ignored for agent completions.").meta({ title: "agent.openrouter.OutputMode" });
var AgentOpenrouterProviderQuantizationSchema = zod.z.union([zod.z.literal("int4").describe("4-bit integer quantization.").meta({ "variantTitle": "Int4" }), zod.z.literal("int8").describe("8-bit integer quantization.").meta({ "variantTitle": "Int8" }), zod.z.literal("fp4").describe("4-bit floating point quantization.").meta({ "variantTitle": "Fp4" }), zod.z.literal("fp6").describe("6-bit floating point quantization.").meta({ "variantTitle": "Fp6" }), zod.z.literal("fp8").describe("8-bit floating point quantization.").meta({ "variantTitle": "Fp8" }), zod.z.literal("fp16").describe("16-bit floating point (half precision).").meta({ "variantTitle": "Fp16" }), zod.z.literal("bf16").describe("16-bit brain floating point.").meta({ "variantTitle": "Bf16" }), zod.z.literal("fp32").describe("32-bit floating point (full precision).").meta({ "variantTitle": "Fp32" }), zod.z.literal("unknown").describe("Unknown quantization level.").meta({ "variantTitle": "Unknown" })]).describe("Model quantization levels for provider filtering.\n\nQuantization reduces model precision to decrease memory usage and\nincrease inference speed, potentially at the cost of output quality.").meta({ title: "agent.openrouter.ProviderQuantization" });

// src/agent/openrouter/provider.ts
var AgentOpenrouterProviderSchema = zod.z.object({
  allow_fallbacks: zod.z.boolean().nullable().describe("Whether to allow fallback to other providers if preferred ones fail.\nDefaults to `true`.").meta({ omitempty: true }).optional(),
  ignore: zod.z.array(zod.z.string()).nullable().describe("Providers to exclude from routing.").meta({ omitempty: true }).optional(),
  only: zod.z.array(zod.z.string()).nullable().describe("Exclusive list of allowed providers. If set, only these providers are used.").meta({ omitempty: true }).optional(),
  order: zod.z.array(zod.z.string()).nullable().describe("Preferred provider order. Earlier providers are tried first.").meta({ omitempty: true }).optional(),
  quantizations: zod.z.array(AgentOpenrouterProviderQuantizationSchema).nullable().describe("Allowed model quantization levels.").meta({ omitempty: true }).optional(),
  require_parameters: zod.z.boolean().nullable().describe("Whether to require that the provider supports all request parameters.\nDefaults to `false`.").meta({ omitempty: true }).optional()
}).describe("Provider routing preferences.\n\nControls which providers are used and in what order when routing\nrequests to upstream model hosts.").meta({ title: "agent.openrouter.Provider" });
var AgentOpenrouterReasoningEffortSchema = zod.z.union([zod.z.literal("none").describe("No reasoning.").meta({ "variantTitle": "None" }), zod.z.literal("minimal").describe("Minimal reasoning effort.").meta({ "variantTitle": "Minimal" }), zod.z.literal("low").describe("Low reasoning effort.").meta({ "variantTitle": "Low" }), zod.z.literal("medium").describe("Medium reasoning effort.").meta({ "variantTitle": "Medium" }), zod.z.literal("high").describe("High reasoning effort.").meta({ "variantTitle": "High" }), zod.z.literal("xhigh").describe("Maximum reasoning effort.").meta({ "variantTitle": "Xhigh" })]).describe("The level of effort the model should put into reasoning.\n\nOnly supported by some models.").meta({ title: "agent.openrouter.ReasoningEffort" });
var AgentOpenrouterReasoningSummaryVerbositySchema = zod.z.union([zod.z.literal("auto").describe("Let the model decide (default, normalized away).").meta({ "variantTitle": "Auto" }), zod.z.literal("concise").describe("Brief summary of reasoning.").meta({ "variantTitle": "Concise" }), zod.z.literal("detailed").describe("Thorough summary of reasoning.").meta({ "variantTitle": "Detailed" })]).describe("Verbosity of the reasoning summary included in responses.\n\nOnly supported by some models.").meta({ title: "agent.openrouter.ReasoningSummaryVerbosity" });

// src/agent/openrouter/reasoning.ts
var AgentOpenrouterReasoningSchema = zod.z.object({
  effort: AgentOpenrouterReasoningEffortSchema.nullable().describe("The reasoning effort level.\n\nOnly supported by some models.").meta({ omitempty: true }).optional(),
  enabled: zod.z.boolean().nullable().describe("Whether reasoning is enabled. Defaults to `true` if other fields are set.").meta({ omitempty: true }).optional(),
  max_tokens: zod.z.number().int().min(0).max(18446744073709552e3).nullable().describe("Maximum tokens for the reasoning/thinking output.\n\nOnly supported by some models.").meta({ omitempty: true }).optional(),
  summary_verbosity: AgentOpenrouterReasoningSummaryVerbositySchema.nullable().describe("Verbosity of reasoning summaries in the response.\n\nOnly supported by some models.").meta({ omitempty: true }).optional()
}).describe('Configuration for model reasoning/thinking capabilities.\n\nSome models (like o1, o3, Claude with extended thinking) support\nexplicit reasoning modes where they can "think" before responding.\nThis struct configures those capabilities.\n\n**Note:** The `max_tokens`, `effort`, and `summary_verbosity` fields are\nonly supported by some models. Unsupported fields are silently ignored.').meta({ title: "agent.openrouter.Reasoning" });
var AgentOpenrouterStopSchema = zod.z.union([zod.z.string().describe("A single stop sequence.").meta({ "variantTitle": "String" }), zod.z.array(zod.z.string()).describe("Multiple stop sequences (up to 4 typically supported).").meta({ "variantTitle": "Strings" })]).describe("Stop sequences that terminate model generation.\n\nWhen the model generates any of these sequences, it immediately\nstops producing further tokens.").meta({ title: "agent.openrouter.Stop" });
var AgentOpenrouterUpstreamSchema = zod.z.literal("openrouter").describe("OpenRouter upstream marker.").meta({ title: "agent.openrouter.Upstream" });
var AgentOpenrouterVerbositySchema = zod.z.union([zod.z.literal("low").describe("Minimal output, concise responses.").meta({ "variantTitle": "Low" }), zod.z.literal("medium").describe("Balanced output (default, normalized away during preparation).").meta({ "variantTitle": "Medium" }), zod.z.literal("high").describe("Detailed output with thorough explanations.").meta({ "variantTitle": "High" }), zod.z.literal("max").describe("Maximum verbosity, most detailed output possible.").meta({ "variantTitle": "Max" })]).describe("The verbosity level for model output.\n\nThis setting hints to the model how detailed its responses should be.\nNot all models support this parameter.").meta({ title: "agent.openrouter.Verbosity" });

// src/agent/openrouter/agentBase.ts
var AgentOpenrouterAgentBaseSchema = zod.z.object({
  client_objectiveai_mcp: AgentClientObjectiveaiMcpSchema.nullable().describe("Client-side ObjectiveAI MCP surface the calling client is\nexpected to expose locally back to the API (objectiveai\nbuilt-in, plus specific plugins / tools by owner+name+version).").meta({ omitempty: true }).optional(),
  context_compression: AgentOpenrouterContextCompressionSchema.nullable().describe("Context compression engine for long contexts. When set, the\nupstream client emits the matching `plugins` entry on the\noutgoing OpenRouter chat-completions request.").meta({ omitempty: true }).optional(),
  frequency_penalty: zod.z.number().min(-34028234663852886e22).max(34028234663852886e22).nullable().describe("Penalizes tokens based on their frequency in the output so far (-2.0 to 2.0).").meta({ omitempty: true }).optional(),
  logit_bias: zod.z.record(zod.z.string(), zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3)).nullable().describe("Token ID to bias mapping (-100 to 100). Positive values increase likelihood.").meta({ omitempty: true }).optional(),
  max_completion_tokens: zod.z.number().int().min(0).max(18446744073709552e3).nullable().describe("Maximum tokens in the completion.").meta({ omitempty: true }).optional(),
  max_tokens: zod.z.number().int().min(0).max(18446744073709552e3).nullable().describe("Maximum tokens (OpenRouter variant of max_completion_tokens).").meta({ omitempty: true }).optional(),
  mcp_servers: zod.z.array(AgentMcpServerSchema).nullable().describe("MCP servers the agent can connect to.").meta({ omitempty: true }).optional(),
  min_p: zod.z.number().min(-34028234663852886e22).max(34028234663852886e22).nullable().describe("Minimum probability threshold for sampling (0.0 to 1.0).").meta({ omitempty: true }).optional(),
  model: zod.z.string().describe('The upstream language model identifier (e.g., `"gpt-4"`, `"claude-3-opus"`).'),
  output_mode: AgentOpenrouterOutputModeSchema.default("instruction").describe("The output mode for vector completions. Ignored for agent completions."),
  post_system_prefix_messages: zod.z.array(AgentCompletionsMessageMessageSchema).nullable().describe("Messages inserted after the leading chain of system/developer messages.").meta({ omitempty: true }).optional(),
  prefix_messages: zod.z.array(AgentCompletionsMessageMessageSchema).nullable().describe("Messages prepended to the user's prompt.").meta({ omitempty: true }).optional(),
  presence_penalty: zod.z.number().min(-34028234663852886e22).max(34028234663852886e22).nullable().describe("Penalizes tokens based on their presence in the output so far (-2.0 to 2.0).").meta({ omitempty: true }).optional(),
  provider: AgentOpenrouterProviderSchema.nullable().describe("Provider routing preferences.").meta({ omitempty: true }).optional(),
  reasoning: AgentOpenrouterReasoningSchema.nullable().describe("Reasoning/thinking configuration for supported models.").meta({ omitempty: true }).optional(),
  repetition_penalty: zod.z.number().min(-34028234663852886e22).max(34028234663852886e22).nullable().describe("Repetition penalty (0.0 to 2.0). Values > 1.0 penalize repetition.").meta({ omitempty: true }).optional(),
  stop: AgentOpenrouterStopSchema.nullable().describe("Stop sequences that halt generation.").meta({ omitempty: true }).optional(),
  suffix_messages: zod.z.array(AgentCompletionsMessageMessageSchema).nullable().describe("Messages appended after the user's prompt.").meta({ omitempty: true }).optional(),
  synthetic_reasoning: zod.z.boolean().nullable().describe("Enable synthetic reasoning for non-reasoning LLMs.\n\n**Vector completions only.** Ignored for agent completions.").meta({ omitempty: true }).optional(),
  temperature: zod.z.number().min(-34028234663852886e22).max(34028234663852886e22).nullable().describe("Sampling temperature (0.0 to 2.0). Higher = more random.").meta({ omitempty: true }).optional(),
  top_a: zod.z.number().min(-34028234663852886e22).max(34028234663852886e22).nullable().describe("Top-a sampling parameter (0.0 to 1.0).").meta({ omitempty: true }).optional(),
  top_k: zod.z.number().int().min(0).max(18446744073709552e3).nullable().describe("Top-k sampling: only consider the k most likely tokens.").meta({ omitempty: true }).optional(),
  top_logprobs: zod.z.number().int().min(0).max(18446744073709552e3).nullable().describe("Number of top log probabilities to return (2-20).\n\n**Vector completions only.** Ignored for agent completions.").meta({ omitempty: true }).optional(),
  top_p: zod.z.number().min(-34028234663852886e22).max(34028234663852886e22).nullable().describe("Nucleus sampling probability (0.0 to 1.0).").meta({ omitempty: true }).optional(),
  upstream: AgentOpenrouterUpstreamSchema.describe("The upstream provider marker."),
  verbosity: AgentOpenrouterVerbositySchema.nullable().describe("Output verbosity hint for supported models.").meta({ omitempty: true }).optional()
}).describe("The base configuration for an OpenRouter Agent (without computed ID).").meta({ title: "agent.openrouter.AgentBase" });

// src/agent/inlineAgentBase.ts
var AgentInlineAgentBaseSchema = zod.z.union([AgentOpenrouterAgentBaseSchema.meta({ "title": "agent.openrouter.AgentBase", "variantTitle": "Openrouter" }), AgentClaudeAgentSdkAgentBaseSchema.meta({ "title": "agent.claude_agent_sdk.AgentBase", "variantTitle": "ClaudeAgentSdk" }), AgentCodexSdkAgentBaseSchema.meta({ "title": "agent.codex_sdk.AgentBase", "variantTitle": "CodexSdk" }), AgentMockAgentBaseSchema.meta({ "title": "agent.mock.AgentBase", "variantTitle": "Mock" })]).describe("The base inline configuration for an Agent (without computed ID or metadata).\n\nThis is an untagged enum that dispatches to the per-upstream AgentBase.\nDeserialization tries each variant in order until one matches.").meta({ title: "agent.InlineAgentBase" });

// src/agent/inlineAgentBaseWithFallbacks.ts
var AgentInlineAgentBaseWithFallbacksSchema = AgentInlineAgentBaseSchema.and(zod.z.object({
  fallbacks: zod.z.array(AgentInlineAgentBaseSchema).nullable().describe("Fallback agents to try if the primary fails.").meta({ omitempty: true }).optional()
})).describe("An [`InlineAgentBase`] with optional fallbacks (no description).").meta({ title: "agent.InlineAgentBaseWithFallbacks" });
var RemotePathCommitOptionalSchema = zod.z.union([zod.z.object({
  commit: zod.z.string().nullable().optional(),
  owner: zod.z.string(),
  remote: zod.z.literal("github"),
  repository: zod.z.string()
}).meta({ "variantTitle": "Github" }), zod.z.object({
  commit: zod.z.string().nullable().optional(),
  owner: zod.z.string(),
  remote: zod.z.literal("filesystem"),
  repository: zod.z.string()
}).meta({ "variantTitle": "Filesystem" }), zod.z.object({
  name: zod.z.string(),
  remote: zod.z.literal("mock")
}).meta({ "variantTitle": "Mock" })]).meta({ title: "RemotePathCommitOptional" });

// src/agent/inlineAgentBaseWithFallbacksOrRemoteCommitOptional.ts
var AgentInlineAgentBaseWithFallbacksOrRemoteCommitOptionalSchema = zod.z.union([AgentInlineAgentBaseWithFallbacksSchema.meta({ "title": "agent.InlineAgentBaseWithFallbacks", "variantTitle": "AgentBase" }), RemotePathCommitOptionalSchema.meta({ "title": "RemotePathCommitOptional", "variantTitle": "Remote" })]).describe("Like [`InlineAgentBaseWithFallbacksOrRemote`] but with optional commit.\nUsed in request types where commit resolution happens server-side.").meta({ title: "agent.InlineAgentBaseWithFallbacksOrRemoteCommitOptional" });

// src/agent/completions/request/agentCompletionCreateParams.ts
var AgentCompletionsRequestAgentCompletionCreateParamsSchema = zod.z.object({
  agent: AgentInlineAgentBaseWithFallbacksOrRemoteCommitOptionalSchema.describe("The agent to use (inline Agent or stored ID)."),
  continuation: zod.z.string().nullable().describe("Continuation from a previous completion, as a base64-encoded string.").meta({ omitempty: true }).optional(),
  messages: zod.z.array(AgentCompletionsMessageMessageSchema).describe("The conversation messages."),
  provider: AgentCompletionsRequestProviderSchema.nullable().describe("Provider routing preferences.").meta({ omitempty: true }).optional(),
  response_format: AgentCompletionsRequestResponseFormatParamSchema.nullable().describe("Output format constraints (text, JSON, or JSON schema).").meta({ omitempty: true }).optional(),
  seed: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3).nullable().describe("Random seed for deterministic generation.").meta({ omitempty: true }).optional(),
  stream: zod.z.boolean().nullable().describe("Whether to stream the response.").meta({ omitempty: true }).optional()
}).describe("Parameters for creating a agent completion.").meta({ title: "agent.completions.request.AgentCompletionCreateParams" });
var FunctionsExecutionsRequestReasoningSchema = zod.z.object({
  agent: AgentInlineAgentBaseWithFallbacksOrRemoteCommitOptionalSchema
}).meta({ title: "functions.executions.request.Reasoning" });
var FunctionsExecutionsRequestStrategySchema = zod.z.union([zod.z.object({
  type: zod.z.literal("default")
}).describe("Scalar or Vector").meta({ "variantTitle": "Default" }), zod.z.object({
  pool: zod.z.number().int().min(0).max(4294967295).nullable().describe("How many vector responses for each execution").optional(),
  rounds: zod.z.number().int().min(0).max(4294967295).nullable().describe("How many sequential rounds of comparison").optional(),
  type: zod.z.literal("swiss_system")
}).describe("Vector").meta({ "variantTitle": "SwissSystem" })]).meta({ title: "functions.executions.request.Strategy" });
var FunctionsExpressionInputValueSchema = zod.z.union([AgentCompletionsMessageRichContentPartSchema.describe("Rich content (image, audio, video, file).").meta({ "title": "agent.completions.message.RichContentPart", "variantTitle": "RichContentPart" }), zod.z.record(zod.z.string(), zod.z.lazy(() => FunctionsExpressionInputValueSchema)).describe("An object with string keys.").meta({ "variantTitle": "Object" }), zod.z.array(zod.z.lazy(() => FunctionsExpressionInputValueSchema)).describe("An array of values.").meta({ "variantTitle": "Array" }), zod.z.string().describe("A string value.").meta({ "variantTitle": "String" }), zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3).describe("An integer value.").meta({ "variantTitle": "Integer" }), zod.z.number().min(-34028234663852886e22).max(34028234663852886e22).describe("A floating-point number.").meta({ "variantTitle": "Number" }), zod.z.boolean().describe("A boolean value.").meta({ "variantTitle": "Boolean" })]).describe("A concrete input value (post-compilation).\n\nRepresents any JSON-like value that can be passed to a Function,\nincluding rich content types (images, audio, video, files).").meta({ title: "functions.expression.InputValue" });
var FunctionsExpressionSpecialSchema = zod.z.union([zod.z.literal("input").describe("Returns the params input as-is.").meta({ "variantTitle": "Input" }), zod.z.literal("output").describe("Returns the params output as-is.").meta({ "variantTitle": "Output" }), zod.z.literal("task_output_l1_normalized").describe("L1-normalizes the output. Scalar/Err pass through.\nVector: L1 normalize. Vectors: L1 normalize each.").meta({ "variantTitle": "TaskOutputL1Normalized" }), zod.z.literal("task_output_weighted_sum").describe("Weighted sum of the output. Vector \u2192 Scalar. Vectors \u2192 Vector.").meta({ "variantTitle": "TaskOutputWeightedSum" }), zod.z.literal("input_items_output_length").describe("Returns the length of input['items'] as u64").meta({ "variantTitle": "InputItemsOutputLength" }), zod.z.literal("input_items_optional_context_split").describe("Splits an input containing items and optionally context into multiple inputs").meta({ "variantTitle": "InputItemsOptionalContextSplit" }), zod.z.literal("input_items_optional_context_merge").describe("Merges multiple inputs containing items and optionally context into a single input").meta({ "variantTitle": "InputItemsOptionalContextMerge" })]).describe("Predefined expression behaviors that require no user-authored code.").meta({ title: "functions.expression.Special" });

// src/functions/expression/expression.ts
var FunctionsExpressionExpressionSchema = zod.z.union([zod.z.object({
  $jmespath: zod.z.string()
}).strict().describe("A JMESPath expression.").meta({ "variantTitle": "JMESPath" }), zod.z.object({
  $starlark: zod.z.string()
}).strict().describe("A Starlark expression.").meta({ "variantTitle": "Starlark" }), zod.z.object({
  $special: FunctionsExpressionSpecialSchema
}).strict().describe("A predefined special expression variant.").meta({ "variantTitle": "Special" })]).describe('An expression that can be either JMESPath or Starlark.\n\nSerializes as `{"$jmespath": "..."}` or `{"$starlark": "..."}` in JSON.\n\n# Examples\n\nJMESPath:\n```json\n{"$jmespath": "input.items[0].name"}\n```\n\nStarlark:\n```json\n{"$starlark": "input[\'items\'][0][\'name\']"}\n```').meta({ title: "functions.expression.Expression" });
var FunctionsExpressionAnyOfInputSchemaSchema = zod.z.object({
  anyOf: zod.z.array(zod.z.lazy(() => FunctionsExpressionInputSchemaSchema)).describe("The possible schemas that the input can match.")
}).describe("Schema for a union of possible types - input must match at least one.").meta({ title: "functions.expression.AnyOfInputSchema" });
var FunctionsExpressionArrayInputSchemaTypeSchema = zod.z.literal("array").meta({ title: "functions.expression.ArrayInputSchemaType" });

// src/functions/expression/arrayInputSchema.ts
var FunctionsExpressionArrayInputSchemaSchema = zod.z.object({
  description: zod.z.string().nullable().describe("Human-readable description of the array.").meta({ omitempty: true }).optional(),
  items: zod.z.lazy(() => FunctionsExpressionInputSchemaSchema).describe("Schema for each item in the array."),
  maxItems: zod.z.number().int().min(0).max(18446744073709552e3).nullable().describe("Maximum number of items allowed.").meta({ omitempty: true }).optional(),
  minItems: zod.z.number().int().min(0).max(18446744073709552e3).nullable().describe("Minimum number of items required.").meta({ omitempty: true }).optional(),
  type: FunctionsExpressionArrayInputSchemaTypeSchema
}).describe("Schema for an array input.").meta({ title: "functions.expression.ArrayInputSchema" });
var FunctionsExpressionAudioInputSchemaTypeSchema = zod.z.literal("audio").meta({ title: "functions.expression.AudioInputSchemaType" });

// src/functions/expression/audioInputSchema.ts
var FunctionsExpressionAudioInputSchemaSchema = zod.z.object({
  description: zod.z.string().nullable().describe("Human-readable description of the expected audio.").meta({ omitempty: true }).optional(),
  type: FunctionsExpressionAudioInputSchemaTypeSchema
}).describe("Schema for an audio input.").meta({ title: "functions.expression.AudioInputSchema" });
var FunctionsExpressionBooleanInputSchemaTypeSchema = zod.z.literal("boolean").meta({ title: "functions.expression.BooleanInputSchemaType" });

// src/functions/expression/booleanInputSchema.ts
var FunctionsExpressionBooleanInputSchemaSchema = zod.z.object({
  description: zod.z.string().nullable().describe("Human-readable description of the boolean.").meta({ omitempty: true }).optional(),
  type: FunctionsExpressionBooleanInputSchemaTypeSchema
}).describe("Schema for a boolean input.").meta({ title: "functions.expression.BooleanInputSchema" });
var FunctionsExpressionFileInputSchemaTypeSchema = zod.z.literal("file").meta({ title: "functions.expression.FileInputSchemaType" });

// src/functions/expression/fileInputSchema.ts
var FunctionsExpressionFileInputSchemaSchema = zod.z.object({
  description: zod.z.string().nullable().describe("Human-readable description of the expected file.").meta({ omitempty: true }).optional(),
  type: FunctionsExpressionFileInputSchemaTypeSchema
}).describe("Schema for a file input.").meta({ title: "functions.expression.FileInputSchema" });
var FunctionsExpressionImageInputSchemaTypeSchema = zod.z.literal("image").meta({ title: "functions.expression.ImageInputSchemaType" });

// src/functions/expression/imageInputSchema.ts
var FunctionsExpressionImageInputSchemaSchema = zod.z.object({
  description: zod.z.string().nullable().describe("Human-readable description of the expected image.").meta({ omitempty: true }).optional(),
  type: FunctionsExpressionImageInputSchemaTypeSchema
}).describe("Schema for an image input (URL or base64-encoded).").meta({ title: "functions.expression.ImageInputSchema" });
var FunctionsExpressionIntegerInputSchemaTypeSchema = zod.z.literal("integer").meta({ title: "functions.expression.IntegerInputSchemaType" });

// src/functions/expression/integerInputSchema.ts
var FunctionsExpressionIntegerInputSchemaSchema = zod.z.object({
  description: zod.z.string().nullable().describe("Human-readable description of the integer.").meta({ omitempty: true }).optional(),
  maximum: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3).nullable().describe("Maximum allowed value (inclusive).").meta({ omitempty: true }).optional(),
  minimum: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3).nullable().describe("Minimum allowed value (inclusive).").meta({ omitempty: true }).optional(),
  type: FunctionsExpressionIntegerInputSchemaTypeSchema
}).describe("Schema for an integer input.").meta({ title: "functions.expression.IntegerInputSchema" });
var FunctionsExpressionNumberInputSchemaTypeSchema = zod.z.literal("number").meta({ title: "functions.expression.NumberInputSchemaType" });

// src/functions/expression/numberInputSchema.ts
var FunctionsExpressionNumberInputSchemaSchema = zod.z.object({
  description: zod.z.string().nullable().describe("Human-readable description of the number.").meta({ omitempty: true }).optional(),
  maximum: zod.z.number().min(-34028234663852886e22).max(34028234663852886e22).nullable().describe("Maximum allowed value (inclusive).").meta({ omitempty: true }).optional(),
  minimum: zod.z.number().min(-34028234663852886e22).max(34028234663852886e22).nullable().describe("Minimum allowed value (inclusive).").meta({ omitempty: true }).optional(),
  type: FunctionsExpressionNumberInputSchemaTypeSchema
}).describe("Schema for a floating-point number input.").meta({ title: "functions.expression.NumberInputSchema" });
var FunctionsExpressionStringInputSchemaTypeSchema = zod.z.literal("string").meta({ title: "functions.expression.StringInputSchemaType" });

// src/functions/expression/stringInputSchema.ts
var FunctionsExpressionStringInputSchemaSchema = zod.z.object({
  description: zod.z.string().nullable().describe("Human-readable description of the string.").meta({ omitempty: true }).optional(),
  enum: zod.z.array(zod.z.string()).nullable().describe("If provided, the string must be one of these values.").meta({ omitempty: true }).optional(),
  type: FunctionsExpressionStringInputSchemaTypeSchema
}).describe("Schema for a string input.").meta({ title: "functions.expression.StringInputSchema" });
var FunctionsExpressionVideoInputSchemaTypeSchema = zod.z.literal("video").meta({ title: "functions.expression.VideoInputSchemaType" });

// src/functions/expression/videoInputSchema.ts
var FunctionsExpressionVideoInputSchemaSchema = zod.z.object({
  description: zod.z.string().nullable().describe("Human-readable description of the expected video.").meta({ omitempty: true }).optional(),
  type: FunctionsExpressionVideoInputSchemaTypeSchema
}).describe("Schema for a video input (URL or base64-encoded).").meta({ title: "functions.expression.VideoInputSchema" });

// src/functions/expression/inputSchema.ts
var FunctionsExpressionInputSchemaSchema = zod.z.union([zod.z.lazy(() => FunctionsExpressionAnyOfInputSchemaSchema).describe("A union of schemas - input must match at least one.").meta({ "title": "functions.expression.AnyOfInputSchema", "variantTitle": "AnyOf" }), zod.z.lazy(() => FunctionsExpressionObjectInputSchemaSchema).describe("An object with named properties.").meta({ "title": "functions.expression.ObjectInputSchema", "variantTitle": "Object" }), zod.z.lazy(() => FunctionsExpressionArrayInputSchemaSchema).describe("An array of items.").meta({ "title": "functions.expression.ArrayInputSchema", "variantTitle": "Array" }), FunctionsExpressionStringInputSchemaSchema.describe("A string value.").meta({ "title": "functions.expression.StringInputSchema", "variantTitle": "String" }), FunctionsExpressionIntegerInputSchemaSchema.describe("An integer value.").meta({ "title": "functions.expression.IntegerInputSchema", "variantTitle": "Integer" }), FunctionsExpressionNumberInputSchemaSchema.describe("A floating-point number.").meta({ "title": "functions.expression.NumberInputSchema", "variantTitle": "Number" }), FunctionsExpressionBooleanInputSchemaSchema.describe("A boolean value.").meta({ "title": "functions.expression.BooleanInputSchema", "variantTitle": "Boolean" }), FunctionsExpressionImageInputSchemaSchema.describe("An image (URL or base64).").meta({ "title": "functions.expression.ImageInputSchema", "variantTitle": "Image" }), FunctionsExpressionAudioInputSchemaSchema.describe("Audio content.").meta({ "title": "functions.expression.AudioInputSchema", "variantTitle": "Audio" }), FunctionsExpressionVideoInputSchemaSchema.describe("Video content.").meta({ "title": "functions.expression.VideoInputSchema", "variantTitle": "Video" }), FunctionsExpressionFileInputSchemaSchema.describe("A file.").meta({ "title": "functions.expression.FileInputSchema", "variantTitle": "File" })]).describe("Schema for validating Function input.\n\nDefines the expected structure and constraints for input data.\nUsed by remote Functions to document and validate their inputs.").meta({ title: "functions.expression.InputSchema" });
var FunctionsExpressionObjectInputSchemaTypeSchema = zod.z.literal("object").meta({ title: "functions.expression.ObjectInputSchemaType" });

// src/functions/expression/objectInputSchema.ts
var FunctionsExpressionObjectInputSchemaSchema = zod.z.object({
  description: zod.z.string().nullable().describe("Human-readable description of the object.").meta({ omitempty: true }).optional(),
  properties: zod.z.record(zod.z.string(), zod.z.lazy(() => FunctionsExpressionInputSchemaSchema)).describe("Schema for each property in the object."),
  required: zod.z.array(zod.z.string()).nullable().describe("List of property names that must be present.").meta({ omitempty: true }).optional(),
  type: FunctionsExpressionObjectInputSchemaTypeSchema
}).describe("Schema for an object input with named properties.").meta({ title: "functions.expression.ObjectInputSchema" });

// src/functions/alpha_scalar/placeholderScalarFunctionTaskExpression.ts
var FunctionsAlphaScalarPlaceholderScalarFunctionTaskExpressionSchema = zod.z.object({
  depth: zod.z.number().int().min(0).max(18446744073709552e3),
  input: FunctionsExpressionExpressionSchema,
  input_schema: FunctionsExpressionObjectInputSchemaSchema,
  max_branch_width: zod.z.number().int().min(0).max(18446744073709552e3),
  max_leaf_width: zod.z.number().int().min(0).max(18446744073709552e3),
  min_branch_width: zod.z.number().int().min(0).max(18446744073709552e3),
  min_leaf_width: zod.z.number().int().min(0).max(18446744073709552e3),
  name: zod.z.string(),
  skip: FunctionsExpressionExpressionSchema.nullable().meta({ omitempty: true }).optional(),
  spec: zod.z.string()
}).meta({ title: "functions.alpha_scalar.PlaceholderScalarFunctionTaskExpression" });
var FunctionsAlphaScalarScalarFunctionTaskExpressionSchema = RemotePathSchema.and(zod.z.object({
  input: FunctionsExpressionExpressionSchema,
  skip: FunctionsExpressionExpressionSchema.nullable().meta({ omitempty: true }).optional()
})).meta({ title: "functions.alpha_scalar.ScalarFunctionTaskExpression" });

// src/functions/alpha_scalar/branchTaskExpression.ts
var FunctionsAlphaScalarBranchTaskExpressionSchema = zod.z.union([FunctionsAlphaScalarScalarFunctionTaskExpressionSchema.and(zod.z.object({
  type: zod.z.literal("alpha.scalar.function")
})).meta({ "variantTitle": "ScalarFunction" }), FunctionsAlphaScalarPlaceholderScalarFunctionTaskExpressionSchema.and(zod.z.object({
  type: zod.z.literal("placeholder.alpha.scalar.function")
})).meta({ "variantTitle": "PlaceholderScalarFunction" })]).meta({ title: "functions.alpha_scalar.BranchTaskExpression" });
var FunctionsAlphaScalarVectorCompletionTaskExpressionSchema = zod.z.object({
  messages: FunctionsExpressionExpressionSchema,
  responses: zod.z.array(AgentCompletionsMessageRichContentSchema),
  skip: FunctionsExpressionExpressionSchema.nullable().meta({ omitempty: true }).optional()
}).meta({ title: "functions.alpha_scalar.VectorCompletionTaskExpression" });

// src/functions/alpha_scalar/leafTaskExpression.ts
var FunctionsAlphaScalarLeafTaskExpressionSchema = FunctionsAlphaScalarVectorCompletionTaskExpressionSchema.and(zod.z.object({
  type: zod.z.literal("vector.completion")
})).meta({ title: "functions.alpha_scalar.LeafTaskExpression" });

// src/functions/alpha_scalar/inlineFunction.ts
var FunctionsAlphaScalarInlineFunctionSchema = zod.z.union([zod.z.object({
  tasks: zod.z.array(FunctionsAlphaScalarBranchTaskExpressionSchema),
  type: zod.z.literal("alpha.scalar.branch.function")
}).meta({ "variantTitle": "Branch" }), zod.z.object({
  tasks: zod.z.array(FunctionsAlphaScalarLeafTaskExpressionSchema),
  type: zod.z.literal("alpha.scalar.leaf.function")
}).meta({ "variantTitle": "Leaf" })]).meta({ title: "functions.alpha_scalar.InlineFunction" });
var FunctionsAlphaVectorPlaceholderScalarFunctionTaskExpressionSchema = zod.z.object({
  depth: zod.z.number().int().min(0).max(18446744073709552e3),
  input: FunctionsExpressionExpressionSchema,
  input_schema: FunctionsExpressionObjectInputSchemaSchema,
  max_branch_width: zod.z.number().int().min(0).max(18446744073709552e3),
  max_leaf_width: zod.z.number().int().min(0).max(18446744073709552e3),
  min_branch_width: zod.z.number().int().min(0).max(18446744073709552e3),
  min_leaf_width: zod.z.number().int().min(0).max(18446744073709552e3),
  name: zod.z.string(),
  skip: FunctionsExpressionExpressionSchema.nullable().meta({ omitempty: true }).optional(),
  spec: zod.z.string()
}).meta({ title: "functions.alpha_vector.PlaceholderScalarFunctionTaskExpression" });
var FunctionsAlphaVectorExpressionVectorFunctionInputSchemaSchema = zod.z.object({
  context: FunctionsExpressionObjectInputSchemaSchema.nullable().meta({ omitempty: true }).optional(),
  items: FunctionsExpressionInputSchemaSchema
}).meta({ title: "functions.alpha_vector.expression.VectorFunctionInputSchema" });
var FunctionsAlphaVectorExpressionVectorFunctionInputValueExpressionSchema = zod.z.object({
  context: FunctionsExpressionExpressionSchema.nullable().meta({ omitempty: true }).optional(),
  items: FunctionsExpressionExpressionSchema
}).meta({ title: "functions.alpha_vector.expression.VectorFunctionInputValueExpression" });

// src/functions/alpha_vector/placeholderVectorFunctionTaskExpression.ts
var FunctionsAlphaVectorPlaceholderVectorFunctionTaskExpressionSchema = zod.z.object({
  depth: zod.z.number().int().min(0).max(18446744073709552e3),
  input: FunctionsAlphaVectorExpressionVectorFunctionInputValueExpressionSchema,
  input_schema: FunctionsAlphaVectorExpressionVectorFunctionInputSchemaSchema,
  max_branch_width: zod.z.number().int().min(0).max(18446744073709552e3),
  max_leaf_width: zod.z.number().int().min(0).max(18446744073709552e3),
  min_branch_width: zod.z.number().int().min(0).max(18446744073709552e3),
  min_leaf_width: zod.z.number().int().min(0).max(18446744073709552e3),
  name: zod.z.string(),
  skip: FunctionsExpressionExpressionSchema.nullable().meta({ omitempty: true }).optional(),
  spec: zod.z.string()
}).meta({ title: "functions.alpha_vector.PlaceholderVectorFunctionTaskExpression" });
var FunctionsAlphaVectorScalarFunctionTaskExpressionSchema = RemotePathSchema.and(zod.z.object({
  input: FunctionsExpressionExpressionSchema,
  skip: FunctionsExpressionExpressionSchema.nullable().meta({ omitempty: true }).optional()
})).meta({ title: "functions.alpha_vector.ScalarFunctionTaskExpression" });
var FunctionsAlphaVectorVectorFunctionTaskExpressionSchema = RemotePathSchema.and(zod.z.object({
  input: FunctionsAlphaVectorExpressionVectorFunctionInputValueExpressionSchema,
  skip: FunctionsExpressionExpressionSchema.nullable().meta({ omitempty: true }).optional()
})).meta({ title: "functions.alpha_vector.VectorFunctionTaskExpression" });

// src/functions/alpha_vector/branchTaskExpression.ts
var FunctionsAlphaVectorBranchTaskExpressionSchema = zod.z.union([FunctionsAlphaVectorScalarFunctionTaskExpressionSchema.and(zod.z.object({
  type: zod.z.literal("alpha.scalar.function")
})).meta({ "variantTitle": "ScalarFunction" }), FunctionsAlphaVectorVectorFunctionTaskExpressionSchema.and(zod.z.object({
  type: zod.z.literal("alpha.vector.function")
})).meta({ "variantTitle": "VectorFunction" }), FunctionsAlphaVectorPlaceholderScalarFunctionTaskExpressionSchema.and(zod.z.object({
  type: zod.z.literal("placeholder.alpha.scalar.function")
})).meta({ "variantTitle": "PlaceholderScalarFunction" }), FunctionsAlphaVectorPlaceholderVectorFunctionTaskExpressionSchema.and(zod.z.object({
  type: zod.z.literal("placeholder.alpha.vector.function")
})).meta({ "variantTitle": "PlaceholderVectorFunction" })]).meta({ title: "functions.alpha_vector.BranchTaskExpression" });
var FunctionsAlphaVectorVectorCompletionTaskExpressionSchema = zod.z.object({
  messages: FunctionsExpressionExpressionSchema,
  responses: FunctionsExpressionExpressionSchema,
  skip: FunctionsExpressionExpressionSchema.nullable().meta({ omitempty: true }).optional()
}).meta({ title: "functions.alpha_vector.VectorCompletionTaskExpression" });

// src/functions/alpha_vector/leafTaskExpression.ts
var FunctionsAlphaVectorLeafTaskExpressionSchema = FunctionsAlphaVectorVectorCompletionTaskExpressionSchema.and(zod.z.object({
  type: zod.z.literal("vector.completion")
})).meta({ title: "functions.alpha_vector.LeafTaskExpression" });

// src/functions/alpha_vector/inlineFunction.ts
var FunctionsAlphaVectorInlineFunctionSchema = zod.z.union([zod.z.object({
  tasks: zod.z.array(FunctionsAlphaVectorBranchTaskExpressionSchema),
  type: zod.z.literal("alpha.vector.branch.function")
}).meta({ "variantTitle": "Branch" }), zod.z.object({
  tasks: zod.z.array(FunctionsAlphaVectorLeafTaskExpressionSchema),
  type: zod.z.literal("alpha.vector.leaf.function")
}).meta({ "variantTitle": "Leaf" })]).meta({ title: "functions.alpha_vector.InlineFunction" });

// src/functions/alphaInlineFunction.ts
var FunctionsAlphaInlineFunctionSchema = zod.z.union([FunctionsAlphaScalarInlineFunctionSchema.meta({ "title": "functions.alpha_scalar.InlineFunction", "variantTitle": "Scalar" }), FunctionsAlphaVectorInlineFunctionSchema.meta({ "title": "functions.alpha_vector.InlineFunction", "variantTitle": "Vector" })]).meta({ title: "functions.AlphaInlineFunction" });
var FunctionsExpressionInputValueExpressionSchema = zod.z.union([AgentCompletionsMessageRichContentPartSchema.describe("Rich content (image, audio, video, file).").meta({ "title": "agent.completions.message.RichContentPart", "variantTitle": "RichContentPart" }), zod.z.record(zod.z.string(), zod.z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), zod.z.lazy(() => FunctionsExpressionInputValueExpressionSchema).describe("A literal value.").meta({ "title": "functions.expression.InputValueExpression", "variantTitle": "Value" })]).describe('A value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```')).describe("An object with values that may be expressions.").meta({ "variantTitle": "Object" }), zod.z.array(zod.z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), zod.z.lazy(() => FunctionsExpressionInputValueExpressionSchema).describe("A literal value.").meta({ "title": "functions.expression.InputValueExpression", "variantTitle": "Value" })]).describe('A value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```')).describe("An array with elements that may be expressions.").meta({ "variantTitle": "Array" }), zod.z.string().describe("A string value.").meta({ "variantTitle": "String" }), zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3).describe("An integer value.").meta({ "variantTitle": "Integer" }), zod.z.number().min(-34028234663852886e22).max(34028234663852886e22).describe("A floating-point number.").meta({ "variantTitle": "Number" }), zod.z.boolean().describe("A boolean value.").meta({ "variantTitle": "Boolean" })]).describe("An input value that may contain expressions (pre-compilation).\n\nSimilar to [`InputValue`] but object values and array elements can be\nexpressions (JMESPath or Starlark) that are evaluated during compilation.").meta({ title: "functions.expression.InputValueExpression" });

// src/functions/placeholderScalarFunctionTaskExpression.ts
var FunctionsPlaceholderScalarFunctionTaskExpressionSchema = zod.z.object({
  input: zod.z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), FunctionsExpressionInputValueExpressionSchema.describe("A literal value.").meta({ "title": "functions.expression.InputValueExpression", "variantTitle": "Value" })]).describe('Expression for the input to pass to the placeholder function.\nReceives: `input`, `map` (if mapped).\n\nA value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```'),
  input_schema: FunctionsExpressionInputSchemaSchema.describe("JSON Schema defining the expected input structure."),
  map: FunctionsExpressionExpressionSchema.nullable().describe("Expression that evaluates to the number of mapped task instances.\nEach instance receives `map` as an integer index (0-based).").meta({ omitempty: true }).optional(),
  output: FunctionsExpressionExpressionSchema.describe("Expression to transform the fixed 0.5 output.\nReceives: `input`, `output` as `Scalar(0.5)`."),
  skip: FunctionsExpressionExpressionSchema.nullable().describe("If this expression evaluates to true, skip the task. Receives: `input`.").meta({ omitempty: true }).optional()
}).describe("Expression for a placeholder scalar function task (pre-compilation).\n\nLike [`ScalarFunctionTaskExpression`] but without owner/repository/commit.\nAlways produces a fixed output of 0.5.").meta({ title: "functions.PlaceholderScalarFunctionTaskExpression" });
var FunctionsPlaceholderVectorFunctionTaskExpressionSchema = zod.z.object({
  input: zod.z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), FunctionsExpressionInputValueExpressionSchema.describe("A literal value.").meta({ "title": "functions.expression.InputValueExpression", "variantTitle": "Value" })]).describe('Expression for the input to pass to the placeholder function.\nReceives: `input`, `map` (if mapped).\n\nA value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```'),
  input_merge: FunctionsExpressionExpressionSchema.describe("Expression merging sub-inputs back into one input.\nReceives: `input` (as an array)."),
  input_schema: FunctionsExpressionInputSchemaSchema.describe("JSON Schema defining the expected input structure."),
  input_split: FunctionsExpressionExpressionSchema.describe("Expression transforming input into sub-inputs for swiss system.\nReceives: `input`."),
  map: FunctionsExpressionExpressionSchema.nullable().describe("Expression that evaluates to the number of mapped task instances.\nEach instance receives `map` as an integer index (0-based).").meta({ omitempty: true }).optional(),
  output: FunctionsExpressionExpressionSchema.describe("Expression to transform the equalized vector output.\nReceives: `input`, `output` as `Vector(equalized)`."),
  output_length: FunctionsExpressionExpressionSchema.describe("Expression computing the expected output vector length.\nReceives: `input`."),
  skip: FunctionsExpressionExpressionSchema.nullable().describe("If this expression evaluates to true, skip the task. Receives: `input`.").meta({ omitempty: true }).optional()
}).describe("Expression for a placeholder vector function task (pre-compilation).\n\nLike [`VectorFunctionTaskExpression`] but without owner/repository/commit.\nAlways produces an equalized vector of length `output_length`.").meta({ title: "functions.PlaceholderVectorFunctionTaskExpression" });
var FunctionsScalarFunctionTaskExpressionSchema = RemotePathSchema.and(zod.z.object({
  input: zod.z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), FunctionsExpressionInputValueExpressionSchema.describe("A literal value.").meta({ "title": "functions.expression.InputValueExpression", "variantTitle": "Value" })]).describe('Expression for the input to pass to the function.\nReceives: `input`, `map` (if mapped).\n\nA value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```'),
  map: FunctionsExpressionExpressionSchema.nullable().describe("Expression that evaluates to the number of mapped task instances.\nEach instance receives `map` as an integer index (0-based).").meta({ omitempty: true }).optional(),
  output: FunctionsExpressionExpressionSchema.describe("Expression to transform the task result into a valid function output.\n\nReceives `output` which is one of 4 variants:\n- `Scalar(Decimal)` - a single score\n- `Vector(Vec<Decimal>)` - a vector of scores\n- `Vectors(Vec<Vec<Decimal>>)` - multiple vectors (from mapped tasks)\n- `Err(Value)` - an error\n\nThe expression must return a `TaskOutputOwned` that is valid for the parent function's type:\n- For scalar functions: must return `Scalar(value)` where value is in [0, 1]\n- For vector functions: must return `Vector(values)` where values sum to ~1 and match the expected length\n\nThe function's final output is computed as a weighted average of all task outputs using\nprofile weights. If a function has only one task, that task's output becomes the function's\noutput directly."),
  skip: FunctionsExpressionExpressionSchema.nullable().describe("If this expression evaluates to true, skip the task. Receives: `input`.").meta({ omitempty: true }).optional()
})).describe("Expression for a task that calls a scalar function (pre-compilation).").meta({ title: "functions.ScalarFunctionTaskExpression" });
var AgentCompletionsMessageAssistantToolCallFunctionExpressionSchema = zod.z.object({
  arguments: zod.z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), zod.z.string().describe("A literal value.").meta({ "variantTitle": "Value" })]).describe('The arguments expression.\n\nA value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```'),
  name: zod.z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), zod.z.string().describe("A literal value.").meta({ "variantTitle": "Value" })]).describe('The function name expression.\n\nA value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```')
}).describe("Expression variant of [`AssistantToolCallFunction`] for dynamic content.").meta({ title: "agent.completions.message.AssistantToolCallFunctionExpression" });

// src/agent/completions/message/assistantToolCallExpression.ts
var AgentCompletionsMessageAssistantToolCallExpressionSchema = zod.z.object({
  function: zod.z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), AgentCompletionsMessageAssistantToolCallFunctionExpressionSchema.describe("A literal value.").meta({ "title": "agent.completions.message.AssistantToolCallFunctionExpression", "variantTitle": "Value" })]).describe('The function expression.\n\nA value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```'),
  id: zod.z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), zod.z.string().describe("A literal value.").meta({ "variantTitle": "Value" })]).describe('The tool call ID expression.\n\nA value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```'),
  type: zod.z.literal("function")
}).describe("A function call expression.").meta({ title: "agent.completions.message.AssistantToolCallExpression" });
var AgentCompletionsMessageRichContentPartExpressionSchema = zod.z.union([zod.z.object({
  text: zod.z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), zod.z.string().describe("A literal value.").meta({ "variantTitle": "Value" })]).describe('A value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```'),
  type: zod.z.literal("text")
}).meta({ "variantTitle": "Text" }), zod.z.object({
  image_url: zod.z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), AgentCompletionsMessageImageUrlSchema.describe("A literal value.").meta({ "title": "agent.completions.message.ImageUrl", "variantTitle": "Value" })]).describe('A value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```'),
  type: zod.z.literal("image_url")
}).meta({ "variantTitle": "ImageUrl" }), zod.z.object({
  input_audio: zod.z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), AgentCompletionsMessageInputAudioSchema.describe("A literal value.").meta({ "title": "agent.completions.message.InputAudio", "variantTitle": "Value" })]).describe('A value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```'),
  type: zod.z.literal("input_audio")
}).meta({ "variantTitle": "InputAudio" }), zod.z.object({
  type: zod.z.literal("input_video"),
  video_url: zod.z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), AgentCompletionsMessageVideoUrlSchema.describe("A literal value.").meta({ "title": "agent.completions.message.VideoUrl", "variantTitle": "Value" })]).describe('A value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```')
}).meta({ "variantTitle": "InputVideo" }), zod.z.object({
  type: zod.z.literal("video_url"),
  video_url: zod.z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), AgentCompletionsMessageVideoUrlSchema.describe("A literal value.").meta({ "title": "agent.completions.message.VideoUrl", "variantTitle": "Value" })]).describe('A value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```')
}).meta({ "variantTitle": "VideoUrl" }), zod.z.object({
  file: zod.z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), AgentCompletionsMessageFileSchema.describe("A literal value.").meta({ "title": "agent.completions.message.File", "variantTitle": "Value" })]).describe('A value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```'),
  type: zod.z.literal("file")
}).meta({ "variantTitle": "File" })]).describe("Expression variant of [`RichContentPart`] for dynamic content.").meta({ title: "agent.completions.message.RichContentPartExpression" });

// src/agent/completions/message/richContentExpression.ts
var AgentCompletionsMessageRichContentExpressionSchema = zod.z.union([zod.z.string().describe("Plain text content.").meta({ "variantTitle": "Text" }), zod.z.array(zod.z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), AgentCompletionsMessageRichContentPartExpressionSchema.describe("A literal value.").meta({ "title": "agent.completions.message.RichContentPartExpression", "variantTitle": "Value" })]).describe('A value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```')).describe("Multi-part content expressions.").meta({ "variantTitle": "Parts" })]).describe("Expression variant of [`RichContent`] for dynamic content.").meta({ title: "agent.completions.message.RichContentExpression" });

// src/agent/completions/message/assistantMessageExpression.ts
var AgentCompletionsMessageAssistantMessageExpressionSchema = zod.z.object({
  content: zod.z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), AgentCompletionsMessageRichContentExpressionSchema.describe("A literal value.").meta({ "title": "agent.completions.message.RichContentExpression", "variantTitle": "Value" })]).describe('A value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```').nullable().describe("The content expression.").meta({ omitempty: true }).optional(),
  name: zod.z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), zod.z.string().describe("A literal value.").meta({ "variantTitle": "Value" })]).describe('A value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```').nullable().meta({ omitempty: true }).optional(),
  reasoning: zod.z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), zod.z.string().describe("A literal value.").meta({ "variantTitle": "Value" })]).describe('A value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```').nullable().meta({ omitempty: true }).optional(),
  refusal: zod.z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), zod.z.string().describe("A literal value.").meta({ "variantTitle": "Value" })]).describe('A value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```').nullable().meta({ omitempty: true }).optional(),
  tool_calls: zod.z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), zod.z.array(zod.z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), AgentCompletionsMessageAssistantToolCallExpressionSchema.describe("A literal value.").meta({ "title": "agent.completions.message.AssistantToolCallExpression", "variantTitle": "Value" })]).describe('A value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```')).describe("A literal value.").meta({ "variantTitle": "Value" })]).describe('A value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```').nullable().meta({ omitempty: true }).optional()
}).describe("Expression variant of [`AssistantMessage`] for dynamic content.").meta({ title: "agent.completions.message.AssistantMessageExpression" });
var AgentCompletionsMessageSimpleContentPartExpressionSchema = zod.z.object({
  text: zod.z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), zod.z.string().describe("A literal value.").meta({ "variantTitle": "Value" })]).describe('The text expression.\n\nA value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```'),
  type: zod.z.literal("text")
}).describe("A text part expression.").meta({ title: "agent.completions.message.SimpleContentPartExpression" });

// src/agent/completions/message/simpleContentExpression.ts
var AgentCompletionsMessageSimpleContentExpressionSchema = zod.z.union([zod.z.string().describe("Plain text content.").meta({ "variantTitle": "Text" }), zod.z.array(zod.z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), AgentCompletionsMessageSimpleContentPartExpressionSchema.describe("A literal value.").meta({ "title": "agent.completions.message.SimpleContentPartExpression", "variantTitle": "Value" })]).describe('A value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```')).describe("Multi-part text content expressions.").meta({ "variantTitle": "Parts" })]).describe("Expression variant of [`SimpleContent`] for dynamic content.").meta({ title: "agent.completions.message.SimpleContentExpression" });

// src/agent/completions/message/developerMessageExpression.ts
var AgentCompletionsMessageDeveloperMessageExpressionSchema = zod.z.object({
  content: zod.z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), AgentCompletionsMessageSimpleContentExpressionSchema.describe("A literal value.").meta({ "title": "agent.completions.message.SimpleContentExpression", "variantTitle": "Value" })]).describe('The message content expression.\n\nA value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```'),
  name: zod.z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), zod.z.string().describe("A literal value.").meta({ "variantTitle": "Value" })]).describe('A value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```').nullable().describe("Optional name expression.").meta({ omitempty: true }).optional()
}).describe("Expression variant of [`DeveloperMessage`] for dynamic content.").meta({ title: "agent.completions.message.DeveloperMessageExpression" });
var AgentCompletionsMessageSystemMessageExpressionSchema = zod.z.object({
  content: zod.z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), AgentCompletionsMessageSimpleContentExpressionSchema.describe("A literal value.").meta({ "title": "agent.completions.message.SimpleContentExpression", "variantTitle": "Value" })]).describe('The message content expression.\n\nA value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```'),
  name: zod.z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), zod.z.string().describe("A literal value.").meta({ "variantTitle": "Value" })]).describe('A value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```').nullable().describe("Optional name expression.").meta({ omitempty: true }).optional()
}).describe("Expression variant of [`SystemMessage`] for dynamic content.").meta({ title: "agent.completions.message.SystemMessageExpression" });
var AgentCompletionsMessageToolMessageExpressionSchema = zod.z.object({
  content: zod.z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), AgentCompletionsMessageRichContentExpressionSchema.describe("A literal value.").meta({ "title": "agent.completions.message.RichContentExpression", "variantTitle": "Value" })]).describe('The content expression.\n\nA value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```'),
  tool_call_id: zod.z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), zod.z.string().describe("A literal value.").meta({ "variantTitle": "Value" })]).describe('The tool call ID expression.\n\nA value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```')
}).describe("Expression variant of [`ToolMessage`] for dynamic content.").meta({ title: "agent.completions.message.ToolMessageExpression" });
var AgentCompletionsMessageUserMessageExpressionSchema = zod.z.object({
  content: zod.z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), AgentCompletionsMessageRichContentExpressionSchema.describe("A literal value.").meta({ "title": "agent.completions.message.RichContentExpression", "variantTitle": "Value" })]).describe('The message content expression.\n\nA value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```'),
  name: zod.z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), zod.z.string().describe("A literal value.").meta({ "variantTitle": "Value" })]).describe('A value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```').nullable().describe("Optional name expression.").meta({ omitempty: true }).optional()
}).describe("Expression variant of [`UserMessage`] for dynamic content.").meta({ title: "agent.completions.message.UserMessageExpression" });

// src/agent/completions/message/messageExpression.ts
var AgentCompletionsMessageMessageExpressionSchema = zod.z.union([AgentCompletionsMessageDeveloperMessageExpressionSchema.and(zod.z.object({
  role: zod.z.literal("developer")
})).meta({ "variantTitle": "Developer" }), AgentCompletionsMessageSystemMessageExpressionSchema.and(zod.z.object({
  role: zod.z.literal("system")
})).meta({ "variantTitle": "System" }), AgentCompletionsMessageUserMessageExpressionSchema.and(zod.z.object({
  role: zod.z.literal("user")
})).meta({ "variantTitle": "User" }), AgentCompletionsMessageAssistantMessageExpressionSchema.and(zod.z.object({
  role: zod.z.literal("assistant")
})).meta({ "variantTitle": "Assistant" }), AgentCompletionsMessageToolMessageExpressionSchema.and(zod.z.object({
  role: zod.z.literal("tool")
})).meta({ "variantTitle": "Tool" })]).describe("A message with expressions for dynamic content.\n\nThis is the expression variant of [`Message`] used in function definitions\nwhere message content can be computed from the function input at runtime.\nSupports both JMESPath and Starlark expressions.").meta({ title: "agent.completions.message.MessageExpression" });

// src/functions/vectorCompletionTaskExpression.ts
var FunctionsVectorCompletionTaskExpressionSchema = zod.z.object({
  map: FunctionsExpressionExpressionSchema.nullable().describe("Expression that evaluates to the number of mapped task instances.\nEach instance receives `map` as an integer index (0-based).").meta({ omitempty: true }).optional(),
  messages: zod.z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), zod.z.array(zod.z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), AgentCompletionsMessageMessageExpressionSchema.describe("A literal value.").meta({ "title": "agent.completions.message.MessageExpression", "variantTitle": "Value" })]).describe('A value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```')).describe("A literal value.").meta({ "variantTitle": "Value" })]).describe('Expression for the conversation messages (the prompt).\nReceives: `input`, `map` (if mapped).\n\nA value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```'),
  output: FunctionsExpressionExpressionSchema.describe("Expression to transform the task result into a valid function output.\n\nReceives `output` as the task's raw result (typically `Vector(scores)`).\n\nThe expression must return a `TaskOutputOwned` that is valid for the parent function's type:\n- For scalar functions: must return `Scalar(value)` where value is in [0, 1]\n- For vector functions: must return `Vector(values)` where values sum to ~1 and match the expected length\n\nThe function's final output is computed as a weighted average of all task outputs using\nprofile weights. If a function has only one task, that task's output becomes the function's\noutput directly."),
  responses: zod.z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), zod.z.array(zod.z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), AgentCompletionsMessageRichContentExpressionSchema.describe("A literal value.").meta({ "title": "agent.completions.message.RichContentExpression", "variantTitle": "Value" })]).describe('A value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```')).describe("A literal value.").meta({ "variantTitle": "Value" })]).describe('Expression for the possible responses the LLMs can vote for.\nReceives: `input`, `map` (if mapped).\n\nA value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```'),
  skip: FunctionsExpressionExpressionSchema.nullable().describe("If this expression evaluates to true, skip the task. Receives: `input`.").meta({ omitempty: true }).optional()
}).describe("Expression for a task that runs a vector completion (pre-compilation).").meta({ title: "functions.VectorCompletionTaskExpression" });
var FunctionsVectorFunctionTaskExpressionSchema = RemotePathSchema.and(zod.z.object({
  input: zod.z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), FunctionsExpressionInputValueExpressionSchema.describe("A literal value.").meta({ "title": "functions.expression.InputValueExpression", "variantTitle": "Value" })]).describe('Expression for the input to pass to the function.\nReceives: `input`, `map` (if mapped).\n\nA value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```'),
  map: FunctionsExpressionExpressionSchema.nullable().describe("Expression that evaluates to the number of mapped task instances.\nEach instance receives `map` as an integer index (0-based).").meta({ omitempty: true }).optional(),
  output: FunctionsExpressionExpressionSchema.describe("Expression to transform the task result into a valid function output.\n\nReceives `output` which is one of 4 variants:\n- `Scalar(Decimal)` - a single score\n- `Vector(Vec<Decimal>)` - a vector of scores\n- `Vectors(Vec<Vec<Decimal>>)` - multiple vectors (from mapped tasks)\n- `Err(Value)` - an error\n\nThe expression must return a `TaskOutputOwned` that is valid for the parent function's type:\n- For scalar functions: must return `Scalar(value)` where value is in [0, 1]\n- For vector functions: must return `Vector(values)` where values sum to ~1 and match the expected length\n\nThe function's final output is computed as a weighted average of all task outputs using\nprofile weights. If a function has only one task, that task's output becomes the function's\noutput directly."),
  skip: FunctionsExpressionExpressionSchema.nullable().describe("If this expression evaluates to true, skip the task. Receives: `input`.").meta({ omitempty: true }).optional()
})).describe("Expression for a task that calls a vector function (pre-compilation).").meta({ title: "functions.VectorFunctionTaskExpression" });

// src/functions/taskExpression.ts
var FunctionsTaskExpressionSchema = zod.z.union([FunctionsScalarFunctionTaskExpressionSchema.and(zod.z.object({
  type: zod.z.literal("scalar.function")
})).meta({ "variantTitle": "ScalarFunction" }), FunctionsVectorFunctionTaskExpressionSchema.and(zod.z.object({
  type: zod.z.literal("vector.function")
})).meta({ "variantTitle": "VectorFunction" }), FunctionsVectorCompletionTaskExpressionSchema.and(zod.z.object({
  type: zod.z.literal("vector.completion")
})).meta({ "variantTitle": "VectorCompletion" }), FunctionsPlaceholderScalarFunctionTaskExpressionSchema.and(zod.z.object({
  type: zod.z.literal("placeholder.scalar.function")
})).meta({ "variantTitle": "PlaceholderScalarFunction" }), FunctionsPlaceholderVectorFunctionTaskExpressionSchema.and(zod.z.object({
  type: zod.z.literal("placeholder.vector.function")
})).meta({ "variantTitle": "PlaceholderVectorFunction" })]).describe("A task definition with expressions (pre-compilation).\n\nTask expressions contain dynamic fields (JMESPath or Starlark) that are\nresolved against input data during compilation. Use [`compile`](Self::compile)\nto produce a concrete [`Task`].").meta({ title: "functions.TaskExpression" });

// src/functions/inlineFunction.ts
var FunctionsInlineFunctionSchema = zod.z.union([zod.z.object({
  tasks: zod.z.array(FunctionsTaskExpressionSchema).describe("The list of tasks to execute. Tasks with a `map` expression are\nexpanded into multiple instances. Each instance is compiled with\n`map` set to the current integer index.\nReceives: `input`, `map` (if mapped)."),
  type: zod.z.literal("scalar.function")
}).describe("Produces a single score in [0, 1].").meta({ "variantTitle": "Scalar" }), zod.z.object({
  input_merge: FunctionsExpressionExpressionSchema.nullable().describe("Expression transforming an array of inputs computed by `input_split`\ninto a single Input object for the Function.\nReceives: `input` (as an array).\nOnly required if the request uses a strategy that needs input splitting.").optional(),
  input_split: FunctionsExpressionExpressionSchema.nullable().describe("Expression transforming input into an input array of the output_length\nWhen the Function is executed with any input from the array,\nThe output_length should be 1.\nReceives: `input`.\nOnly required if the request uses a strategy that needs input splitting.").optional(),
  tasks: zod.z.array(FunctionsTaskExpressionSchema).describe("The list of tasks to execute. Tasks with a `map` expression are\nexpanded into multiple instances. Each instance is compiled with\n`map` set to the current integer index.\nReceives: `input`, `map` (if mapped)."),
  type: zod.z.literal("vector.function")
}).describe("Produces a vector of scores that sums to 1.").meta({ "variantTitle": "Vector" })]).describe("An inline function definition without metadata.\n\nUsed when embedding function logic directly in requests rather than\nreferencing a remote function. Lacks description and input\nschema fields.").meta({ title: "functions.InlineFunction" });

// src/functions/fullInlineFunction.ts
var FunctionsFullInlineFunctionSchema = zod.z.union([FunctionsAlphaInlineFunctionSchema.meta({ "title": "functions.AlphaInlineFunction", "variantTitle": "Alpha" }), FunctionsInlineFunctionSchema.meta({ "title": "functions.InlineFunction", "variantTitle": "Standard" })]).meta({ title: "functions.FullInlineFunction" });

// src/functions/fullInlineFunctionOrRemoteCommitOptional.ts
var FunctionsFullInlineFunctionOrRemoteCommitOptionalSchema = zod.z.union([FunctionsFullInlineFunctionSchema.meta({ "title": "functions.FullInlineFunction", "variantTitle": "Inline" }), RemotePathCommitOptionalSchema.meta({ "title": "RemotePathCommitOptional", "variantTitle": "Remote" })]).describe("A function specification that is either a full inline function definition\nor a remote path reference.").meta({ title: "functions.FullInlineFunctionOrRemoteCommitOptional" });
var FunctionsTaskProfileSchema = zod.z.union([RemotePathSchema.describe("Profile for a nested function task (references another profile).").meta({ "title": "RemotePath", "variantTitle": "Remote" }), zod.z.lazy(() => FunctionsInlineProfileSchema).describe("Inline profile for a task (tasks-based or auto).").meta({ "title": "functions.InlineProfile", "variantTitle": "Inline" }), zod.z.object({}).describe("Placeholder task \u2014 no configuration needed, output is fixed.").meta({ "variantTitle": "Placeholder" })]).describe("Configuration for a single task within a Profile.\n\nEach variant corresponds to a task type in the Function definition.").meta({ title: "functions.TaskProfile" });
var WeightsEntrySchema = zod.z.object({
  invert: zod.z.boolean().nullable().describe("If true, invert this agent's vote distribution before combining.\n\nWhen omitted or false, the vote distribution is used as-is.").meta({ omitempty: true }).optional(),
  weight: zod.z.number().min(-34028234663852886e22).max(34028234663852886e22).describe("The weight for this agent in the swarm. Must be in [0, 1].")
}).describe("An entry in weights with an explicit weight and optional invert flag.").meta({ title: "WeightsEntry" });

// src/weights.ts
var WeightsSchema = zod.z.union([zod.z.array(zod.z.number().min(-34028234663852886e22).max(34028234663852886e22)).describe("Simple vector of decimal weights.").meta({ "variantTitle": "Weights" }), zod.z.array(WeightsEntrySchema).describe("Vector of entries with optional invert flags.").meta({ "variantTitle": "Entries" })]).describe("Weights for a swarm's agents.\n\n- `Weights(Vec<Decimal>)` - simple representation (no inversion)\n- `Entries(Vec<WeightsEntry>)` - weights with optional per-agent `invert`").meta({ title: "Weights" });

// src/functions/inlineTasksProfile.ts
var FunctionsInlineTasksProfileSchema = zod.z.object({
  tasks: zod.z.array(zod.z.lazy(() => FunctionsTaskProfileSchema)).describe("Configuration for each task in the corresponding Function."),
  weights: WeightsSchema.nullable().describe("Optional weights for each Task in the corresponding Function.\nIf `None`, uniform weights are used.").meta({ omitempty: true }).optional()
}).describe("An inline tasks-based profile definition without metadata.").meta({ title: "functions.InlineTasksProfile" });
var AgentInlineAgentBaseWithFallbacksOrRemoteSchema = zod.z.union([AgentInlineAgentBaseWithFallbacksSchema.meta({ "title": "agent.InlineAgentBaseWithFallbacks", "variantTitle": "AgentBase" }), RemotePathSchema.meta({ "title": "RemotePath", "variantTitle": "Remote" })]).describe("An agent specification that is either an inline agent base with fallbacks\nor a remote path reference.\n\nUsed in swarm definitions to allow agents to be specified inline\n(with optional fallbacks) or resolved from a remote source via a\nhashmap during conversion.").meta({ title: "agent.InlineAgentBaseWithFallbacksOrRemote" });

// src/agent/inlineAgentBaseWithFallbacksOrRemoteWithCount.ts
var AgentInlineAgentBaseWithFallbacksOrRemoteWithCountSchema = AgentInlineAgentBaseWithFallbacksOrRemoteSchema.and(zod.z.object({
  count: zod.z.number().int().min(0).max(18446744073709552e3).default(1).describe("Number of instances of this agent in the swarm. Defaults to 1.")
})).describe("An [`InlineAgentBaseWithFallbacksOrRemote`] with a count\n(pre-validation swarm agent slot).").meta({ title: "agent.InlineAgentBaseWithFallbacksOrRemoteWithCount" });

// src/swarm/inlineSwarmBase.ts
var SwarmInlineSwarmBaseSchema = zod.z.object({
  agents: zod.z.array(AgentInlineAgentBaseWithFallbacksOrRemoteWithCountSchema).describe("The LLMs in this swarm, with optional counts and fallbacks."),
  weights: WeightsSchema.nullable().describe("Optional weights for each agent. If `None`, uniform weights are used.").meta({ omitempty: true }).optional()
}).describe("An inline swarm base definition (without computed ID or metadata).\n\nContains a list of agent configurations that will be validated, deduplicated,\nand sorted when converting to an [`InlineSwarm`].").meta({ title: "swarm.InlineSwarmBase" });

// src/functions/inlineProfile.ts
var FunctionsInlineProfileSchema = zod.z.union([zod.z.lazy(() => FunctionsInlineTasksProfileSchema).describe("Tasks-based profile with per-task configuration.").meta({ "title": "functions.InlineTasksProfile", "variantTitle": "Tasks" }), SwarmInlineSwarmBaseSchema.describe("Auto profile that applies a single swarm+weights to all vector completion tasks.").meta({ "title": "swarm.InlineSwarmBase", "variantTitle": "Auto" })]).describe("An inline profile, either tasks-based or auto.").meta({ title: "functions.InlineProfile" });

// src/functions/inlineProfileOrRemoteCommitOptional.ts
var FunctionsInlineProfileOrRemoteCommitOptionalSchema = zod.z.union([FunctionsInlineProfileSchema.meta({ "title": "functions.InlineProfile", "variantTitle": "Inline" }), RemotePathCommitOptionalSchema.meta({ "title": "RemotePathCommitOptional", "variantTitle": "Remote" })]).describe("A profile specification that is either an inline profile definition\nor a remote path reference.").meta({ title: "functions.InlineProfileOrRemoteCommitOptional" });

// src/functions/executions/request/functionExecutionCreateParams.ts
var FunctionsExecutionsRequestFunctionExecutionCreateParamsSchema = zod.z.object({
  continuation: zod.z.string().nullable().describe("Continuation from a previous completion, as a base64-encoded string.").meta({ omitempty: true }).optional(),
  from_cache: zod.z.boolean().nullable().meta({ omitempty: true }).optional(),
  function: FunctionsFullInlineFunctionOrRemoteCommitOptionalSchema.describe("The function to execute (inline definition or remote path)."),
  input: FunctionsExpressionInputValueSchema,
  invert: zod.z.boolean().nullable().describe('If `true`, invert every output in the streamed response *after* the\ninner function has finished computing \u2014 scalar outputs become\n`1 - x`, vector outputs are reversed in place. The expression\nevaluator inside the function still sees the original scores; only\nthe chunks delivered to the client (and the aggregated response\npassed to the usage handler) are inverted. Useful when a function\nis naturally written to score "lower is better" but the consumer\nwants "higher is better", or vice versa.').meta({ omitempty: true }).optional(),
  profile: FunctionsInlineProfileOrRemoteCommitOptionalSchema.describe("The profile to use (inline definition or remote path)."),
  provider: AgentCompletionsRequestProviderSchema.nullable().meta({ omitempty: true }).optional(),
  reasoning: FunctionsExecutionsRequestReasoningSchema.nullable().meta({ omitempty: true }).optional(),
  retry_token: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  seed: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3).nullable().meta({ omitempty: true }).optional(),
  split: zod.z.boolean().nullable().meta({ omitempty: true }).optional(),
  strategy: FunctionsExecutionsRequestStrategySchema.nullable().meta({ omitempty: true }).optional(),
  stream: zod.z.boolean().nullable().meta({ omitempty: true }).optional()
}).describe("Parameters for creating a function execution.").meta({ title: "functions.executions.request.FunctionExecutionCreateParams" });
var SwarmInlineSwarmBaseOrRemoteCommitOptionalSchema = zod.z.union([SwarmInlineSwarmBaseSchema.meta({ "title": "swarm.InlineSwarmBase", "variantTitle": "SwarmBase" }), RemotePathCommitOptionalSchema.meta({ "title": "RemotePathCommitOptional", "variantTitle": "Remote" })]).describe("Like [`InlineSwarmBaseOrRemote`] but with optional commit.\nUsed in request types where commit resolution happens server-side.").meta({ title: "swarm.InlineSwarmBaseOrRemoteCommitOptional" });

// src/vector/completions/request/vectorCompletionCreateParams.ts
var VectorCompletionsRequestVectorCompletionCreateParamsSchema = zod.z.object({
  continuation: zod.z.string().nullable().describe("Continuation from a previous completion, as a base64-encoded string.").meta({ omitempty: true }).optional(),
  from_cache: zod.z.boolean().nullable().describe("If true, uses cached votes when available.").meta({ omitempty: true }).optional(),
  messages: zod.z.array(AgentCompletionsMessageMessageSchema).describe("The conversation messages (the prompt)."),
  provider: AgentCompletionsRequestProviderSchema.nullable().describe("Provider routing preferences.").meta({ omitempty: true }).optional(),
  responses: zod.z.array(AgentCompletionsMessageRichContentSchema).describe("The possible responses the LLMs can vote for."),
  retry: zod.z.string().nullable().describe("If present, reuses votes from a previous request with this ID.").meta({ omitempty: true }).optional(),
  seed: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3).nullable().describe("Random seed for deterministic results.").meta({ omitempty: true }).optional(),
  stream: zod.z.boolean().nullable().describe("Whether to stream the response.").meta({ omitempty: true }).optional(),
  swarm: SwarmInlineSwarmBaseOrRemoteCommitOptionalSchema.describe("The Swarm of agents to use.")
}).describe("Parameters for creating a vector completion.\n\nVector completions run multiple agent completions (one per LLM in the\nswarm), force each to vote for one of the predefined responses, and\ncombine votes using the provided profile weights to produce final scores.").meta({ title: "vector.completions.request.VectorCompletionCreateParams" });

// src/cli/command/agents/logs/read/id/response.ts
var CliCommandAgentsLogsReadIdResponseSchema = zod.z.union([zod.z.object({
  body: AgentCompletionsRequestAgentCompletionCreateParamsSchema,
  created_at: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  response_id: zod.z.string(),
  sender_agent_instance_hierarchy: zod.z.string(),
  type: zod.z.literal("agent_completion_request")
}).meta({ "variantTitle": "AgentCompletionRequest" }), zod.z.object({
  body: VectorCompletionsRequestVectorCompletionCreateParamsSchema,
  created_at: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  response_id: zod.z.string(),
  sender_agent_instance_hierarchy: zod.z.string(),
  type: zod.z.literal("vector_completion_request")
}).meta({ "variantTitle": "VectorCompletionRequest" }), zod.z.object({
  body: FunctionsExecutionsRequestFunctionExecutionCreateParamsSchema,
  created_at: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  response_id: zod.z.string(),
  sender_agent_instance_hierarchy: zod.z.string(),
  type: zod.z.literal("function_execution_request")
}).meta({ "variantTitle": "FunctionExecutionRequest" }), zod.z.object({
  index: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  response_id: zod.z.string(),
  tool_call_id: zod.z.string(),
  type: zod.z.literal("tool_response")
}).meta({ "variantTitle": "ToolResponse" }), zod.z.object({
  arguments: zod.z.string(),
  function_name: zod.z.string().describe("Function name from the openai tool_call payload\n(`tool_calls[i].function.name`)."),
  index: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  response_id: zod.z.string(),
  tool_call_id: zod.z.string(),
  tool_call_index: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  type: zod.z.literal("response_tool_calls")
}).meta({ "variantTitle": "ResponseToolCalls" }), zod.z.object({
  type: zod.z.literal("text")
}).meta({ "variantTitle": "Text" }), AgentCompletionsMessageImageUrlSchema.and(zod.z.object({
  type: zod.z.literal("image")
})).meta({ "variantTitle": "Image" }), AgentCompletionsMessageInputAudioSchema.and(zod.z.object({
  type: zod.z.literal("audio")
})).meta({ "variantTitle": "Audio" }), AgentCompletionsMessageVideoUrlSchema.and(zod.z.object({
  type: zod.z.literal("video")
})).meta({ "variantTitle": "Video" }), AgentCompletionsMessageFileSchema.and(zod.z.object({
  type: zod.z.literal("file")
})).meta({ "variantTitle": "File" })]).describe('Resolved payload for one `logs.messages."index"`. Tagged by\n`type`, snake_case discriminant. The MCP projection in\n[`CommandResponse::into_mcp`] hands media variants over as\n[`ContentBlock`]s and text as a bare JSON string \u2014 matching the\nexisting `agents queue read id` projection of `RichContentPart`.\nThe five non-content variants render as JSONL with their full\ntyped body so callers can introspect request-blob /\ntool-response / tool-call metadata.\n\n[`ContentBlock`]: crate::mcp::tool::ContentBlock').meta({ title: "cli.command.agents.logs.read.id.Response" });

// src/viewer/command/agents/logs/read/id.ts
async function agentsLogsReadIdExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/logs/read/id" }), zod.z.union([CliErrorSchema, CliCommandAgentsLogsReadIdResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read id: cli produced no output before the end marker");
  }
  return first;
}
async function agentsLogsReadIdExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/logs/read/id" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read id: cli produced no output before the end marker");
  }
  return first;
}
async function agentsLogsReadIdRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/logs/read/id/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read id request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsLogsReadIdRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/logs/read/id/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read id request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsLogsReadIdResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/logs/read/id/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read id response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsLogsReadIdResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/logs/read/id/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read id response_schema: cli produced no output before the end marker");
  }
  return first;
}
function agentsLogsReadPendingExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/logs/read/pending" }), zod.z.union([CliErrorSchema, CliCommandAgentsLogsReadAllResponseItemSchema]));
}
function agentsLogsReadPendingExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/logs/read/pending" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function agentsLogsReadPendingRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/logs/read/pending/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read pending request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsLogsReadPendingRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/logs/read/pending/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read pending request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsLogsReadPendingResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/logs/read/pending/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read pending response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsLogsReadPendingResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/logs/read/pending/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read pending response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandAgentsLogsReadSubscribeAgentsInactiveTagSchema = zod.z.literal("agents_inactive").describe('Single-variant enum whose lone variant serializes as the\nliteral string `"agents_inactive"`. Construct as\n`AgentsInactiveTag::AgentsInactive`; the surrounding\n`ResponseItem::AgentsInactive(_)` carries it.').meta({ title: "cli.command.agents.logs.read.subscribe.AgentsInactiveTag" });

// src/cli/command/agents/logs/read/subscribe/responseItem.ts
var CliCommandAgentsLogsReadSubscribeResponseItemSchema = zod.z.union([CliCommandAgentsLogsReadAllResponseItemSchema.meta({ "title": "cli.command.agents.logs.read.all.ResponseItem", "variantTitle": "Item" }), CliCommandAgentsLogsReadSubscribeAgentsInactiveTagSchema.meta({ "title": "cli.command.agents.logs.read.subscribe.AgentsInactiveTag", "variantTitle": "AgentsInactive" })]).describe('Subscribe\'s wire shape. Either a real parts-grouped block\n(the EXACT same enum `read all` / `read pending` emit) OR\nthe literal string `"agents_inactive"`. `#[serde(untagged)]`\nso the `Item` arm passes through transparently \u2014 JSONL\nconsumers see either a block JSON object or a bare string.').meta({ title: "cli.command.agents.logs.read.subscribe.ResponseItem" });

// src/viewer/command/agents/logs/read/subscribe.ts
function agentsLogsReadSubscribeExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/logs/read/subscribe" }), zod.z.union([CliErrorSchema, CliCommandAgentsLogsReadSubscribeResponseItemSchema]));
}
function agentsLogsReadSubscribeExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/logs/read/subscribe" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function agentsLogsReadSubscribeRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/logs/read/subscribe/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsLogsReadSubscribeRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/logs/read/subscribe/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsLogsReadSubscribeResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/logs/read/subscribe/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsLogsReadSubscribeResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/logs/read/subscribe/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandAgentsMessageResponseSchema = zod.z.union([zod.z.object({
  type: zod.z.literal("delivered")
}).describe("The queue row reached a live agent (the API stamped its id\nonto an assistant chunk's `request_message_ids`) before\nany other race finalized.").meta({ "variantTitle": "Delivered" }), zod.z.object({
  agent_instance_hierarchy: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  agent_tag: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  id: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  type: zod.z.literal("enqueued")
}).describe("The target's tag wasn't bound at call time (PENDING /\nABSENT). The message was deferred into the queue.").meta({ "variantTitle": "Enqueued" }), zod.z.object({
  agent_instance_hierarchy: zod.z.string(),
  type: zod.z.literal("id")
}).describe("The stream=false path re-execed itself as a detached\nsubprocess (stream=true) and the subprocess yielded a\n`ResponseItem::Id` first. Same payload as spawn's\n`ResponseItem::Id(String)` \u2014 the bare\n`agent_instance_hierarchy` string the runner just minted.").meta({ "variantTitle": "Id" })]).describe('Unary response (stream=false). Exactly one of these per call.\nInternally tagged via `type`; bare unit variant `Delivered`\nserializes as `{"type":"delivered"}`.').meta({ title: "cli.command.agents.message.Response" });
var AgentCompletionsMessageAssistantToolCallFunctionDeltaSchema = zod.z.object({
  arguments: zod.z.string().nullable().describe("The arguments being streamed (accumulated across deltas).").meta({ omitempty: true }).optional(),
  name: zod.z.string().nullable().describe("The function name (only present in the first delta).").meta({ omitempty: true }).optional()
}).describe("Function call details in a streaming tool call.").meta({ title: "agent.completions.message.AssistantToolCallFunctionDelta" });
var AgentCompletionsMessageAssistantToolCallTypeSchema = zod.z.literal("function").describe("A function call.").meta({ title: "agent.completions.message.AssistantToolCallType" });

// src/agent/completions/message/assistantToolCallDelta.ts
var AgentCompletionsMessageAssistantToolCallDeltaSchema = zod.z.object({
  function: AgentCompletionsMessageAssistantToolCallFunctionDeltaSchema.nullable().describe("The function call details.").meta({ omitempty: true }).optional(),
  id: zod.z.string().nullable().describe("The unique ID of this tool call.").meta({ omitempty: true }).optional(),
  index: zod.z.number().int().min(0).max(18446744073709552e3).describe("The index of this tool call."),
  type: AgentCompletionsMessageAssistantToolCallTypeSchema.nullable().describe('The type of tool call (always "function").').meta({ omitempty: true }).optional()
}).describe("A tool call delta in a streaming response.").meta({ title: "agent.completions.message.AssistantToolCallDelta" });
var AgentCompletionsResponseAssistantRoleSchema = zod.z.literal("assistant").describe("The assistant role.").meta({ title: "agent.completions.response.AssistantRole" });
var AgentCompletionsResponseFinishReasonSchema = zod.z.union([zod.z.literal("stop").describe("The model reached a natural stop point or stop sequence.").meta({ "variantTitle": "Stop" }), zod.z.literal("length").describe("The model reached the maximum token limit.").meta({ "variantTitle": "Length" }), zod.z.literal("tool_calls").describe("The model decided to call one or more tools.").meta({ "variantTitle": "ToolCalls" }), zod.z.literal("content_filter").describe("The response was filtered due to content policy.").meta({ "variantTitle": "ContentFilter" }), zod.z.literal("error").describe("An error occurred during generation.").meta({ "variantTitle": "Error" })]).describe("The reason the model stopped generating.").meta({ title: "agent.completions.response.FinishReason" });
var AgentCompletionsResponseTopLogprobSchema = zod.z.object({
  bytes: zod.z.array(zod.z.number().int().min(0).max(255)).nullable().describe("The raw bytes of the token.").optional(),
  logprob: zod.z.number().min(-34028234663852886e22).max(34028234663852886e22).nullable().describe("The log probability of this token.").optional(),
  token: zod.z.string().describe("The token string.")
}).describe("A top alternative token with its log probability.").meta({ title: "agent.completions.response.TopLogprob" });

// src/agent/completions/response/logprob.ts
var AgentCompletionsResponseLogprobSchema = zod.z.object({
  bytes: zod.z.array(zod.z.number().int().min(0).max(255)).nullable().describe("The raw bytes of the token.").optional(),
  logprob: zod.z.number().min(-34028234663852886e22).max(34028234663852886e22).describe("The log probability of this token."),
  token: zod.z.string().describe("The token string."),
  top_logprobs: zod.z.array(AgentCompletionsResponseTopLogprobSchema).describe("The top alternative tokens and their log probabilities.")
}).describe("Log probability information for a single token.").meta({ title: "agent.completions.response.Logprob" });

// src/agent/completions/response/logprobs.ts
var AgentCompletionsResponseLogprobsSchema = zod.z.object({
  content: zod.z.array(AgentCompletionsResponseLogprobSchema).nullable().describe("Log probabilities for content tokens.").optional(),
  refusal: zod.z.array(AgentCompletionsResponseLogprobSchema).nullable().describe("Log probabilities for refusal tokens.").optional()
}).describe("Log probabilities for generated tokens.").meta({ title: "agent.completions.response.Logprobs" });
var AgentCompletionsResponseCompletionTokensDetailsSchema = zod.z.object({
  accepted_prediction_tokens: zod.z.number().int().min(0).max(18446744073709552e3).nullable().describe("Tokens from accepted predictions (speculative decoding).").meta({ omitempty: true }).optional(),
  audio_tokens: zod.z.number().int().min(0).max(18446744073709552e3).nullable().describe("Audio output tokens.").meta({ omitempty: true }).optional(),
  reasoning_tokens: zod.z.number().int().min(0).max(18446744073709552e3).nullable().describe("Tokens used for reasoning/thinking.").meta({ omitempty: true }).optional(),
  rejected_prediction_tokens: zod.z.number().int().min(0).max(18446744073709552e3).nullable().describe("Tokens from rejected predictions (speculative decoding).").meta({ omitempty: true }).optional()
}).describe("Detailed breakdown of completion token usage.").meta({ title: "agent.completions.response.CompletionTokensDetails" });
var AgentCompletionsResponseCostDetailsSchema = zod.z.object({
  upstream_inference_cost: zod.z.number().min(-34028234663852886e22).max(34028234663852886e22).describe("Cost charged by the immediate upstream (e.g., OpenRouter)."),
  upstream_upstream_inference_cost: zod.z.number().min(-34028234663852886e22).max(34028234663852886e22).describe("Cost charged by the upstream's upstream (e.g., the actual model provider).")
}).describe("Detailed cost breakdown.").meta({ title: "agent.completions.response.CostDetails" });
var AgentCompletionsResponsePromptTokensDetailsSchema = zod.z.object({
  audio_tokens: zod.z.number().int().min(0).max(18446744073709552e3).nullable().describe("Audio input tokens.").meta({ omitempty: true }).optional(),
  cache_write_tokens: zod.z.number().int().min(0).max(18446744073709552e3).nullable().describe("Tokens written to cache.").meta({ omitempty: true }).optional(),
  cached_tokens: zod.z.number().int().min(0).max(18446744073709552e3).nullable().describe("Tokens served from cache.").meta({ omitempty: true }).optional(),
  video_tokens: zod.z.number().int().min(0).max(18446744073709552e3).nullable().describe("Video input tokens.").meta({ omitempty: true }).optional()
}).describe("Detailed breakdown of prompt token usage.").meta({ title: "agent.completions.response.PromptTokensDetails" });

// src/agent/completions/response/upstreamUsage.ts
var AgentCompletionsResponseUpstreamUsageSchema = zod.z.object({
  completion_tokens: zod.z.number().int().min(0).max(18446744073709552e3).describe("Number of tokens in the completion."),
  completion_tokens_details: AgentCompletionsResponseCompletionTokensDetailsSchema.nullable().describe("Detailed breakdown of completion tokens.").meta({ omitempty: true }).optional(),
  cost: zod.z.number().min(-34028234663852886e22).max(34028234663852886e22).describe("The cost charged by ObjectiveAI for this request."),
  cost_details: AgentCompletionsResponseCostDetailsSchema.nullable().describe("Detailed cost breakdown.").meta({ omitempty: true }).optional(),
  cost_multiplier: zod.z.number().min(-34028234663852886e22).max(34028234663852886e22).describe("The multiplier applied to compute ObjectiveAI's charge."),
  is_byok: zod.z.boolean().describe("Whether this request used Bring Your Own Key (BYOK)."),
  prompt_tokens: zod.z.number().int().min(0).max(18446744073709552e3).describe("Number of tokens in the prompt."),
  prompt_tokens_details: AgentCompletionsResponsePromptTokensDetailsSchema.nullable().describe("Detailed breakdown of prompt tokens.").meta({ omitempty: true }).optional(),
  total_cost: zod.z.number().min(-34028234663852886e22).max(34028234663852886e22).describe("Total cost including ObjectiveAI's charge plus all upstream charges.\nFor BYOK requests, ObjectiveAI only charges the cost_multiplier difference,\nbut total_cost still includes what the upstream provider charged."),
  total_tokens: zod.z.number().int().min(0).max(18446744073709552e3).describe("Total tokens (prompt + completion).")
}).describe("Token usage and cost information from an upstream provider.\n\nThis is the per-assistant-response usage yielded by upstream clients.\nIt includes upstream-specific fields like `cost_multiplier` and `is_byok`.").meta({ title: "agent.completions.response.UpstreamUsage" });

// src/agent/completions/response/streaming/assistantResponseChunk.ts
var AgentCompletionsResponseStreamingAssistantResponseChunkSchema = zod.z.object({
  content: AgentCompletionsMessageRichContentSchema.nullable().meta({ omitempty: true }).optional(),
  created: zod.z.number().int().min(0).max(18446744073709552e3),
  finish_reason: AgentCompletionsResponseFinishReasonSchema.nullable().optional(),
  index: zod.z.number().int().min(0).max(18446744073709552e3),
  logprobs: AgentCompletionsResponseLogprobsSchema.nullable().meta({ omitempty: true }).optional(),
  model: zod.z.string(),
  provider: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  reasoning: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  refusal: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  request_message_ids: zod.z.array(zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3)).nullable().describe("`message_queue_contents.id`s the API consumed to seed this\nturn's request. Stamped onto the first assistant chunk the\nAPI emits downstream \u2014 when set, the consumer owns these\nrows. Kinds are resolved CLI-side at write time (SQL CASE\nagainst `message_queue_contents.kind`), so the wire stays\n`i64`-only. Both `None` and `Some(empty)` are skipped on\nserialize.").meta({ omitempty: true }).optional(),
  role: AgentCompletionsResponseAssistantRoleSchema,
  service_tier: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  system_fingerprint: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  tool_calls: zod.z.array(AgentCompletionsMessageAssistantToolCallDeltaSchema).nullable().meta({ omitempty: true }).optional(),
  upstream_id: zod.z.string(),
  usage: AgentCompletionsResponseUpstreamUsageSchema.nullable().describe("Upstream usage for this assistant response (set by upstream clients).").meta({ omitempty: true }).optional()
}).describe("A chunk of a streaming agent completion response.\n\nMultiple chunks are received via Server-Sent Events and can be\naccumulated into a complete [`AgentCompletion`](response::unary::AgentCompletion)\nusing the [`push`](Self::push) method.").meta({ title: "agent.completions.response.streaming.AssistantResponseChunk" });
var AgentCompletionsResponseToolRoleSchema = zod.z.literal("tool").meta({ title: "agent.completions.response.ToolRole" });

// src/agent/completions/response/toolResponse.ts
var AgentCompletionsResponseToolResponseSchema = zod.z.object({
  content: AgentCompletionsMessageRichContentSchema.describe("The content of the tool response."),
  index: zod.z.number().int().min(0).max(18446744073709552e3),
  metadata: AgentCompletionsMessageToolResponseMetadataSchema.nullable().describe("Optional vendor-extension metadata, populated by\n`objectiveai-mcp-proxy` via MCP's `_meta` extension bag.").meta({ omitempty: true }).optional(),
  request_message_ids: zod.z.array(zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3)).nullable().describe("Mirrors `AssistantResponseChunk.request_message_ids` \u2014\nthe consumed `message_queue_contents.id`s. Currently never\npopulated (the API stamps the assistant chunk instead);\nthe field exists so the wire shape is symmetric across\nthe two `MessageChunk` variants. Both `None` and\n`Some(empty)` are skipped on serialize.").meta({ omitempty: true }).optional(),
  role: AgentCompletionsResponseToolRoleSchema,
  tool_call_id: zod.z.string().describe("The ID of the tool call this message responds to.")
}).describe("A tool message containing the result of a tool call.").meta({ title: "agent.completions.response.ToolResponse" });

// src/agent/completions/response/streaming/messageChunk.ts
var AgentCompletionsResponseStreamingMessageChunkSchema = zod.z.union([AgentCompletionsResponseStreamingAssistantResponseChunkSchema.meta({ "title": "agent.completions.response.streaming.AssistantResponseChunk", "variantTitle": "Assistant" }), AgentCompletionsResponseToolResponseSchema.meta({ "title": "agent.completions.response.ToolResponse", "variantTitle": "Tool" })]).meta({ title: "agent.completions.response.streaming.MessageChunk" });
var AgentCompletionsResponseStreamingObjectSchema = zod.z.literal("agent.completion.chunk").describe("A agent completion chunk object.").meta({ title: "agent.completions.response.streaming.Object" });
var AgentCompletionsResponseUsageSchema = zod.z.object({
  completion_tokens: zod.z.number().int().min(0).max(18446744073709552e3).describe("Total tokens generated across all assistant responses."),
  completion_tokens_details: AgentCompletionsResponseCompletionTokensDetailsSchema.nullable().describe("Breakdown of completion tokens (reasoning, audio, etc.) if available.").meta({ omitempty: true }).optional(),
  cost: zod.z.number().min(-34028234663852886e22).max(34028234663852886e22).describe("Cost charged by ObjectiveAI for this request."),
  cost_details: AgentCompletionsResponseCostDetailsSchema.nullable().describe("Breakdown of upstream and upstream_upstream costs if available.").meta({ omitempty: true }).optional(),
  prompt_tokens: zod.z.number().int().min(0).max(18446744073709552e3).describe("Total prompt tokens across all assistant responses."),
  prompt_tokens_details: AgentCompletionsResponsePromptTokensDetailsSchema.nullable().describe("Breakdown of prompt tokens (cached, audio, etc.) if available.").meta({ omitempty: true }).optional(),
  total_cost: zod.z.number().min(-34028234663852886e22).max(34028234663852886e22).describe("Total cost including upstream provider charges. Only differs from `cost`\nwhen using BYOK (Bring Your Own Key)."),
  total_tokens: zod.z.number().int().min(0).max(18446744073709552e3).describe("Sum of completion and prompt tokens.")
}).describe('Aggregated token and cost usage for an agent completion.\n\nThis is the "primary" usage type that aggregates across all upstream\nassistant responses within a single agent completion.').meta({ title: "agent.completions.response.Usage" });
var AgentUpstreamSchema = zod.z.union([zod.z.literal("unknown").describe("Unknown Upstream.").meta({ "variantTitle": "Unknown" }), zod.z.literal("openrouter").describe("OpenRouter Upstream.").meta({ "variantTitle": "Openrouter" }), zod.z.literal("claude_agent_sdk").describe("Claude Agent SDK Upstream.").meta({ "variantTitle": "ClaudeAgentSdk" }), zod.z.literal("codex_sdk").describe("Codex SDK Upstream.").meta({ "variantTitle": "CodexSdk" }), zod.z.literal("mock").describe("Mock Upstream.").meta({ "variantTitle": "Mock" })]).describe("Supported agent upstreams.").meta({ title: "agent.Upstream" });
var ErrorResponseErrorSchema = zod.z.object({
  code: zod.z.number().int().min(0).max(65535).describe("The HTTP status code of the error response."),
  message: JsonValueSchema.describe("The error message or details as a JSON value.")
}).describe("An error returned by the ObjectiveAI API.\n\nThis struct represents an API error response containing an HTTP status\ncode and a message. The message can be any JSON value, allowing for\nboth simple string errors and structured error objects.").meta({ title: "error.ResponseError" });

// src/agent/completions/response/streaming/agentCompletionChunk.ts
var AgentCompletionsResponseStreamingAgentCompletionChunkSchema = zod.z.object({
  agent_full_id: zod.z.string().describe("WF-level id: concatenation of the primary agent's id with all\nfallback ids (see `InlineAgentWithFallbacks::full_id`). Same\nfor every slot in the same WF request."),
  agent_id: zod.z.string().describe("Leaf agent id of the slot that produced this chunk. For the\nprimary attempt this is the primary agent's id; on fallback it\nis the fallback agent's id. Same on every chunk of a slot."),
  agent_instance_hierarchy: zod.z.string().describe("Full agent instance hierarchy for this completion's slot \u2014\n`{ctx lineage}/{agent_full_id}-{response_id}`, or the fixed\ncontinuation value on resume. Same on every chunk of a slot."),
  agent_remote: RemotePathSchema.nullable().describe("`RemotePath` the WF was fetched from. `None` when the WF was\nsupplied inline. Same for every slot in the same WF request.").meta({ omitempty: true }).optional(),
  continuation: zod.z.string().nullable().describe("Continuation state for multi-turn conversations (only present in the final chunk).").meta({ omitempty: true }).optional(),
  created: zod.z.number().int().min(0).max(18446744073709552e3),
  error: ErrorResponseErrorSchema.nullable().describe("Error details if this completion failed.").meta({ omitempty: true }).optional(),
  id: zod.z.string(),
  messages: zod.z.array(AgentCompletionsResponseStreamingMessageChunkSchema),
  messages_queued: zod.z.boolean().nullable().describe("`true` when the MCP proxy holds queued messages that were not\ndelivered to the agent via a tool response on this turn. Only\nset when `continuation` is also set \u2014 the caller acts on it by\nissuing the continuation. Absent when nothing is queued, when\nthere is no continuation to act on, or when the peek failed\n(the failure is surfaced via `error`).").meta({ omitempty: true }).optional(),
  object: AgentCompletionsResponseStreamingObjectSchema.describe('The object type (always "agent.completion.chunk").'),
  upstream: AgentUpstreamSchema.describe("Upstream provider"),
  usage: AgentCompletionsResponseUsageSchema.nullable().describe("Token usage (only present in the final chunk).").meta({ omitempty: true }).optional()
}).describe("A chunk of a streaming agent completion response.\n\nMultiple chunks are received via Server-Sent Events and can be\naccumulated into a complete [`AgentCompletion`](response::unary::AgentCompletion)\nusing the [`push`](Self::push) method.").meta({ title: "agent.completions.response.streaming.AgentCompletionChunk" });

// src/cli/command/agents/message/responseItem.ts
var CliCommandAgentsMessageResponseItemSchema = zod.z.union([zod.z.object({
  type: zod.z.literal("delivered")
}).meta({ "variantTitle": "Delivered" }), zod.z.object({
  agent_instance_hierarchy: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  agent_tag: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  id: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  type: zod.z.literal("enqueued")
}).meta({ "variantTitle": "Enqueued" }), zod.z.object({
  agent_instance_hierarchy: zod.z.string(),
  type: zod.z.literal("id")
}).meta({ "variantTitle": "Id" }), AgentCompletionsResponseStreamingAgentCompletionChunkSchema.and(zod.z.object({
  type: zod.z.literal("chunk")
})).describe('Newtype-of-struct under an internally-tagged enum: the\nchunk\'s own fields land at the top level of the JSON, with\n`"type":"chunk"` injected. Wire shape equivalent to spawn\'s\n`ResponseItem::Chunk(AgentCompletionChunk)` plus the `type`\ndiscriminator.').meta({ "variantTitle": "Chunk" })]).describe("Streamed response (stream=true). The cli yields a sequence of\nthese. Same `Delivered` / `Enqueued` / `Id` first-item\nsemantics as [`Response`]; the spawn-take-over branch adds\nstreaming `Chunk` items after the initial `Id`.").meta({ title: "cli.command.agents.message.ResponseItem" });

// src/viewer/command/agents/message.ts
function agentsMessageExecuteStreaming(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, dangerous_advanced: { ...request.dangerous_advanced ?? {}, stream: true }, path_type: "agents/message" }), zod.z.union([CliErrorSchema, CliCommandAgentsMessageResponseItemSchema]));
}
function agentsMessageExecuteStreamingJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, dangerous_advanced: { ...request.dangerous_advanced ?? {}, stream: true }, path_type: "agents/message" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function agentsMessageExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, dangerous_advanced: request.dangerous_advanced ? { ...request.dangerous_advanced, stream: void 0 } : request.dangerous_advanced, path_type: "agents/message" }), zod.z.union([CliErrorSchema, CliCommandAgentsMessageResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents message: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMessageExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, dangerous_advanced: request.dangerous_advanced ? { ...request.dangerous_advanced, stream: void 0 } : request.dangerous_advanced, path_type: "agents/message" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents message: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMessageRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/message/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents message request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMessageRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/message/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents message request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMessageResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/message/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents message response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMessageResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/message/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents message response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandAgentsPublishResponseSchema = zod.z.object({
  sha: zod.z.string()
}).meta({ title: "cli.command.agents.publish.Response" });

// src/viewer/command/agents/publish.ts
async function agentsPublishExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/publish" }), zod.z.union([CliErrorSchema, CliCommandAgentsPublishResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents publish: cli produced no output before the end marker");
  }
  return first;
}
async function agentsPublishExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/publish" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents publish: cli produced no output before the end marker");
  }
  return first;
}
async function agentsPublishRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/publish/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents publish request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsPublishRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/publish/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents publish request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsPublishResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/publish/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents publish response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsPublishResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/publish/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents publish response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandAgentsQueueDeleteResponseSchema = zod.z.object({
  agent_instance_hierarchy: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  agent_tag: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  content: AgentCompletionsMessageRichContentSchema,
  enqueued_at: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  id: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  key: zod.z.string().nullable().describe("Idempotency token, if the dropped row had one.").meta({ omitempty: true }).optional()
}).describe("What was deleted. Carries every column of the original\n`prompts` row so the caller can confirm the drop:\nexactly one of `agent_instance_hierarchy` / `agent_tag` is set\n(matching the original target), `enqueued_at` is the original\nunix-seconds timestamp, and `content` is the reconstructed\n`RichContent` body.").meta({ title: "cli.command.agents.queue.delete.Response" });

// src/viewer/command/agents/queue/delete.ts
async function agentsQueueDeleteExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/queue/delete" }), zod.z.union([CliErrorSchema, CliCommandAgentsQueueDeleteResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue delete: cli produced no output before the end marker");
  }
  return first;
}
async function agentsQueueDeleteExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/queue/delete" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue delete: cli produced no output before the end marker");
  }
  return first;
}
async function agentsQueueDeleteRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/queue/delete/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue delete request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsQueueDeleteRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/queue/delete/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue delete request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsQueueDeleteResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/queue/delete/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue delete response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsQueueDeleteResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/queue/delete/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue delete response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandAgentsQueueDeliverAgentActiveTypeSchema = zod.z.literal("AgentActive").meta({ title: "cli.command.agents.queue.deliver.AgentActiveType" });

// src/cli/command/agents/queue/deliver/agentActiveResponseItem.ts
var CliCommandAgentsQueueDeliverAgentActiveResponseItemSchema = zod.z.object({
  agent_instance_hierarchy: zod.z.string(),
  type: CliCommandAgentsQueueDeliverAgentActiveTypeSchema
}).describe("This agent's lock was held by a live owner \u2014 it is already active\nand will drain its own queue; nothing was spawned for it.").meta({ title: "cli.command.agents.queue.deliver.AgentActiveResponseItem" });
var CliCommandAgentsQueueDeliverAgentSpawnedTypeSchema = zod.z.literal("AgentSpawned").meta({ title: "cli.command.agents.queue.deliver.AgentSpawnedType" });

// src/cli/command/agents/queue/deliver/agentSpawnedResponseItem.ts
var CliCommandAgentsQueueDeliverAgentSpawnedResponseItemSchema = zod.z.object({
  agent_instance_hierarchy: zod.z.string(),
  type: CliCommandAgentsQueueDeliverAgentSpawnedTypeSchema
}).describe("This agent's lock was won and its spawn has started; its output\nfollows as [`ValueResponseItem`]s (in `stream_spawns` mode).").meta({ title: "cli.command.agents.queue.deliver.AgentSpawnedResponseItem" });
var CliCommandAgentsQueueDeliverAllAgentsActiveSchema = zod.z.literal("AllAgentsActive").describe('Every target has resolved to active-or-spawned. Wire shape is the\nbare string `"AllAgentsActive"` (a one-variant enum \u2014 a unit\nvariant in the untagged [`ResponseItem`] would serialize as\n`null`, not the marker string). The detached default mode stops\nreading its child at this item.').meta({ title: "cli.command.agents.queue.deliver.AllAgentsActive" });
var CliCommandAgentsQueueDeliverValueResponseItemSchema = zod.z.object({
  agent_instance_hierarchy: zod.z.string().describe("The delivered agent's `agent_instance_hierarchy`."),
  value: JsonValueSchema.describe("The typed root item the spawn emitted.")
}).describe("One output item from one delivered agent's spawn stream. `value`\nis the typed root [`crate::cli::command::ResponseItem`] (the spawn\nitem wrapped at the root) \u2014 boxed because the root union\ntransitively contains *this* type (`agents \u2192 queue \u2192 deliver`),\nand boxing is what makes the recursion sized.\n\nThe `value` field's JSON schema is opaqued to `serde_json::Value`\n(renders as bare `{}` aka JsonValue) so the published schema\ndoesn't inline the entire root union \u2014 that's the TS7056 blowup\nthe root and tier aggregates dodge by being `json_schema_ignore`.\nDownstream SDKs see `value: JsonValue` on the typed `execute`\npath; consumers that want to peer inside parse it case-by-case.").meta({ title: "cli.command.agents.queue.deliver.ValueResponseItem" });

// src/cli/command/agents/queue/deliver/responseItem.ts
var CliCommandAgentsQueueDeliverResponseItemSchema = zod.z.union([CliCommandAgentsQueueDeliverValueResponseItemSchema.meta({ "title": "cli.command.agents.queue.deliver.ValueResponseItem", "variantTitle": "Value" }), CliCommandAgentsQueueDeliverAgentActiveResponseItemSchema.meta({ "title": "cli.command.agents.queue.deliver.AgentActiveResponseItem", "variantTitle": "AgentActive" }), CliCommandAgentsQueueDeliverAgentSpawnedResponseItemSchema.meta({ "title": "cli.command.agents.queue.deliver.AgentSpawnedResponseItem", "variantTitle": "AgentSpawned" }), CliCommandAgentsQueueDeliverAllAgentsActiveSchema.meta({ "title": "cli.command.agents.queue.deliver.AllAgentsActive", "variantTitle": "AllAgentsActive" })]).describe('One stream item from `agents queue deliver`. Untagged \u2014 the\nvariants are disjoint on the wire: `Value` requires `value`,\n`AgentActive` / `AgentSpawned` carry distinct `type` markers, and\n`AllAgentsActive` is the bare string `"AllAgentsActive"`.').meta({ title: "cli.command.agents.queue.deliver.ResponseItem" });

// src/viewer/command/agents/queue/deliver.ts
function agentsQueueDeliverExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/queue/deliver" }), zod.z.union([CliErrorSchema, CliCommandAgentsQueueDeliverResponseItemSchema]));
}
function agentsQueueDeliverExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/queue/deliver" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function agentsQueueDeliverRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/queue/deliver/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue deliver request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsQueueDeliverRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/queue/deliver/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue deliver request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsQueueDeliverResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/queue/deliver/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue deliver response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsQueueDeliverResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/queue/deliver/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue deliver response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsQueueReadIdExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/queue/read/id" }), zod.z.union([CliErrorSchema, AgentCompletionsMessageRichContentPartSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue read id: cli produced no output before the end marker");
  }
  return first;
}
async function agentsQueueReadIdExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/queue/read/id" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue read id: cli produced no output before the end marker");
  }
  return first;
}
async function agentsQueueReadIdRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/queue/read/id/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue read id request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsQueueReadIdRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/queue/read/id/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue read id request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsQueueReadIdResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/queue/read/id/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue read id response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsQueueReadIdResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/queue/read/id/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue read id response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandAgentsQueueReadPendingQueuePartSchema = zod.z.object({
  id: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  type: CliCommandAgentsLogsReadAllClientNotificationPartTypeSchema
}).describe("One row inside a `ResponseItem` block \u2014 a\n`message_queue_contents` entry. The `id` is the\n`message_queue_contents.id`, which you pass to\n`agents queue read id <n>` to drill into the body.\n`timestamp_queued` is on the enclosing block, not here\n(one block = one `message_queue` parent row, sharing one\n`enqueued_at`).").meta({ title: "cli.command.agents.queue.read.pending.QueuePart" });

// src/cli/command/agents/queue/read/pending/responseItem.ts
var CliCommandAgentsQueueReadPendingResponseItemSchema = zod.z.union([zod.z.object({
  agent_instance_hierarchy: zod.z.string(),
  by: zod.z.literal("agent_instance_hierarchy"),
  delete_id: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3).describe("`message_queue.id` \u2014 the row-level id this block\nrepresents. Pass to `agents queue delete <id>` to\nsoft-flip the entire row (all parts) in one call.\nDistinct from each `QueuePart.id` (which is a\n`message_queue_contents.id` for drilling into one\ncontent slot via `agents queue read id`)."),
  key: zod.z.string().nullable().describe("Idempotency token, if the row was enqueued with `--key`.").meta({ omitempty: true }).optional(),
  parts: zod.z.array(CliCommandAgentsQueueReadPendingQueuePartSchema),
  sender_agent_instance_hierarchy: zod.z.string().describe("AIH of the caller who enqueued \u2014 from\n`message_queue.sender_*`."),
  timestamp_queued: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3).describe("`message_queue.enqueued_at`. One block = one parent\n`message_queue` row, so this is well-defined\nblock-level.")
}).meta({ "variantTitle": "AgentInstanceHierarchy" }), zod.z.object({
  agent_tag: zod.z.string(),
  by: zod.z.literal("tag"),
  delete_id: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3).describe("`message_queue.id`. Pass to `agents queue delete <id>`."),
  key: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  parts: zod.z.array(CliCommandAgentsQueueReadPendingQueuePartSchema),
  sender_agent_instance_hierarchy: zod.z.string(),
  timestamp_queued: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3)
}).meta({ "variantTitle": "Tag" })]).describe("One pending `message_queue` row, with its content rows\ngrouped as `parts`. Two variants \u2014 direct AIH target or tag\ntarget \u2014 both flat (no nested `LookupState`).").meta({ title: "cli.command.agents.queue.read.pending.ResponseItem" });

// src/viewer/command/agents/queue/read/pending.ts
function agentsQueueReadPendingExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/queue/read/pending" }), zod.z.union([CliErrorSchema, CliCommandAgentsQueueReadPendingResponseItemSchema]));
}
function agentsQueueReadPendingExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/queue/read/pending" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function agentsQueueReadPendingRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/queue/read/pending/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue read pending request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsQueueReadPendingRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/queue/read/pending/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue read pending request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsQueueReadPendingResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/queue/read/pending/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue read pending response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsQueueReadPendingResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/queue/read/pending/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue read pending response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandAgentsSpawnResponseItemSchema = zod.z.union([AgentCompletionsResponseStreamingAgentCompletionChunkSchema.meta({ "title": "agent.completions.response.streaming.AgentCompletionChunk", "variantTitle": "Chunk" }), zod.z.string().meta({ "variantTitle": "Id" })]).meta({ title: "cli.command.agents.spawn.ResponseItem" });

// src/viewer/command/agents/spawn.ts
function agentsSpawnExecuteStreaming(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, dangerous_advanced: { ...request.dangerous_advanced ?? {}, stream: true }, path_type: "agents/spawn" }), zod.z.union([CliErrorSchema, CliCommandAgentsSpawnResponseItemSchema]));
}
function agentsSpawnExecuteStreamingJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, dangerous_advanced: { ...request.dangerous_advanced ?? {}, stream: true }, path_type: "agents/spawn" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function agentsSpawnExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, dangerous_advanced: request.dangerous_advanced ? { ...request.dangerous_advanced, stream: void 0 } : request.dangerous_advanced, path_type: "agents/spawn" }), zod.z.union([CliErrorSchema, zod.z.string()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents spawn: cli produced no output before the end marker");
  }
  return first;
}
async function agentsSpawnExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, dangerous_advanced: request.dangerous_advanced ? { ...request.dangerous_advanced, stream: void 0 } : request.dangerous_advanced, path_type: "agents/spawn" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents spawn: cli produced no output before the end marker");
  }
  return first;
}
async function agentsSpawnRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/spawn/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents spawn request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsSpawnRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/spawn/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents spawn request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsSpawnResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/spawn/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents spawn response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsSpawnResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/spawn/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents spawn response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandAgentsSpawnAgentSpecSchema = zod.z.union([AgentInlineAgentBaseWithFallbacksOrRemoteCommitOptionalSchema.meta({ "title": "agent.InlineAgentBaseWithFallbacksOrRemoteCommitOptional", "variantTitle": "Resolved" }), zod.z.string().meta({ "variantTitle": "Favorite" })]).describe("CLI-surface form for the `--agent` / `--agent-inline` argument: either\na fully resolved inline-or-remote spec, or a bare favorite name that\nthe CLI resolves to one of those at handler time. Untagged: an inline\nagent object or a remote-path object deserializes into `Resolved`; a\nbare JSON string lands on `Favorite`.").meta({ title: "cli.command.agents.spawn.AgentSpec" });

// src/cli/command/agents/tags/apply/response.ts
var CliCommandAgentsTagsApplyResponseSchema = zod.z.union([zod.z.object({
  agent_instance: zod.z.string(),
  agent_instance_hierarchy: zod.z.string().describe("`{parent}/{agent_instance}`."),
  by: zod.z.literal("agent_instance"),
  name: zod.z.string(),
  parent_agent_instance_hierarchy: zod.z.string()
}).meta({ "variantTitle": "AgentInstance" }), zod.z.object({
  agent_spec: CliCommandAgentsSpawnAgentSpecSchema,
  by: zod.z.literal("agent"),
  name: zod.z.string(),
  parent_agent_instance_hierarchy: zod.z.string(),
  tag_group_id: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3)
}).meta({ "variantTitle": "Agent" }), zod.z.union([zod.z.object({
  agent_instance_hierarchy: zod.z.string(),
  agent_tag: zod.z.string(),
  by: zod.z.literal("agent_tag"),
  name: zod.z.string(),
  state: zod.z.literal("bound")
}).meta({ "variantTitle": "Bound" }), zod.z.object({
  agent_spec: CliCommandAgentsSpawnAgentSpecSchema,
  agent_tag: zod.z.string(),
  by: zod.z.literal("agent_tag"),
  name: zod.z.string(),
  parent_agent_instance_hierarchy: zod.z.string(),
  state: zod.z.literal("grouped"),
  tag_group_id: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3)
}).meta({ "variantTitle": "Grouped" })]).describe('Wire shape mirrors the resolved source state:\n`{"by":"agent_tag","name":...,"agent_tag":...,"state":"bound"|"grouped", \u2026}`.').meta({ "variantTitle": "AgentTag" })]).describe("Apply response. Each variant carries the freshly-applied state\nso callers don't need a follow-up lookup.").meta({ title: "cli.command.agents.tags.apply.Response" });

// src/viewer/command/agents/tags/apply.ts
async function agentsTagsApplyExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/tags/apply" }), zod.z.union([CliErrorSchema, CliCommandAgentsTagsApplyResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tags apply: cli produced no output before the end marker");
  }
  return first;
}
async function agentsTagsApplyExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/tags/apply" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tags apply: cli produced no output before the end marker");
  }
  return first;
}
async function agentsTagsApplyRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/tags/apply/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tags apply request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsTagsApplyRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/tags/apply/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tags apply request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsTagsApplyResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/tags/apply/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tags apply response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsTagsApplyResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/tags/apply/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tags apply response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsTagsLookupRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/tags/lookup/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tags lookup request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsTagsLookupRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/tags/lookup/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tags lookup request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsTagsLookupResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/tags/lookup/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tags lookup response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsTagsLookupResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/tags/lookup/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tags lookup response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandOkSchema = zod.z.literal("Ok").describe('Success-only response shape. Wire form is the bare string `"Ok"` \u2014\na single-variant enum gives us a typed sentinel that serializes and\ndeserializes through serde as the static string. Used as `Response`\non every cli leaf whose only success signal is "it worked."').meta({ title: "cli.command.Ok" });

// src/viewer/command/config/agents/favorites/add.ts
async function configAgentsFavoritesAddExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/agents/favorites/add" }), zod.z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents favorites add: cli produced no output before the end marker");
  }
  return first;
}
async function configAgentsFavoritesAddExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/agents/favorites/add" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents favorites add: cli produced no output before the end marker");
  }
  return first;
}
async function configAgentsFavoritesAddRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/agents/favorites/add/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents favorites add request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configAgentsFavoritesAddRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/agents/favorites/add/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents favorites add request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configAgentsFavoritesAddResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/agents/favorites/add/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents favorites add response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configAgentsFavoritesAddResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/agents/favorites/add/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents favorites add response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configAgentsFavoritesDelExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/agents/favorites/del" }), zod.z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents favorites del: cli produced no output before the end marker");
  }
  return first;
}
async function configAgentsFavoritesDelExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/agents/favorites/del" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents favorites del: cli produced no output before the end marker");
  }
  return first;
}
async function configAgentsFavoritesDelRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/agents/favorites/del/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents favorites del request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configAgentsFavoritesDelRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/agents/favorites/del/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents favorites del request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configAgentsFavoritesDelResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/agents/favorites/del/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents favorites del response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configAgentsFavoritesDelResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/agents/favorites/del/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents favorites del response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configAgentsFavoritesEditExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/agents/favorites/edit" }), zod.z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents favorites edit: cli produced no output before the end marker");
  }
  return first;
}
async function configAgentsFavoritesEditExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/agents/favorites/edit" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents favorites edit: cli produced no output before the end marker");
  }
  return first;
}
async function configAgentsFavoritesEditRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/agents/favorites/edit/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents favorites edit request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configAgentsFavoritesEditRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/agents/favorites/edit/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents favorites edit request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configAgentsFavoritesEditResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/agents/favorites/edit/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents favorites edit response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configAgentsFavoritesEditResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/agents/favorites/edit/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents favorites edit response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandConfigAgentsFavoritesGetResponseItemSchema = zod.z.union([zod.z.object({
  commit: zod.z.string().nullable().optional(),
  name: zod.z.string(),
  note: zod.z.string(),
  owner: zod.z.string(),
  remote: zod.z.literal("github"),
  repository: zod.z.string()
}).meta({ "variantTitle": "Github" }), zod.z.object({
  commit: zod.z.string().nullable().optional(),
  name: zod.z.string(),
  note: zod.z.string(),
  owner: zod.z.string(),
  remote: zod.z.literal("filesystem"),
  repository: zod.z.string()
}).meta({ "variantTitle": "Filesystem" }), zod.z.object({
  name: zod.z.string(),
  note: zod.z.string(),
  remote: zod.z.literal("mock")
}).meta({ "variantTitle": "Mock" })]).meta({ title: "cli.command.config.agents.favorites.get.ResponseItem" });

// src/viewer/command/config/agents/favorites/get.ts
function configAgentsFavoritesGetExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/agents/favorites/get" }), zod.z.union([CliErrorSchema, CliCommandConfigAgentsFavoritesGetResponseItemSchema]));
}
function configAgentsFavoritesGetExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/agents/favorites/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function configAgentsFavoritesGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/agents/favorites/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents favorites get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configAgentsFavoritesGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/agents/favorites/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents favorites get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configAgentsFavoritesGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/agents/favorites/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents favorites get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configAgentsFavoritesGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/agents/favorites/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents favorites get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandConfigAgentsGetResponseSchema = zod.z.object({
  favorites: zod.z.array(CliCommandConfigAgentsFavoritesGetResponseItemSchema).nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.config.agents.get.Response" });

// src/viewer/command/config/agents/get.ts
async function configAgentsGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/agents/get" }), zod.z.union([CliErrorSchema, CliCommandConfigAgentsGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents get: cli produced no output before the end marker");
  }
  return first;
}
async function configAgentsGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/agents/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents get: cli produced no output before the end marker");
  }
  return first;
}
async function configAgentsGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/agents/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configAgentsGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/agents/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configAgentsGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/agents/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configAgentsGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/agents/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsFavoritesAddExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/functions/favorites/add" }), zod.z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions favorites add: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsFavoritesAddExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/functions/favorites/add" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions favorites add: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsFavoritesAddRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/favorites/add/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions favorites add request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsFavoritesAddRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/favorites/add/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions favorites add request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsFavoritesAddResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/favorites/add/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions favorites add response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsFavoritesAddResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/favorites/add/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions favorites add response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsFavoritesDelExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/functions/favorites/del" }), zod.z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions favorites del: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsFavoritesDelExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/functions/favorites/del" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions favorites del: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsFavoritesDelRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/favorites/del/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions favorites del request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsFavoritesDelRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/favorites/del/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions favorites del request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsFavoritesDelResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/favorites/del/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions favorites del response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsFavoritesDelResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/favorites/del/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions favorites del response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsFavoritesEditExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/functions/favorites/edit" }), zod.z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions favorites edit: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsFavoritesEditExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/functions/favorites/edit" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions favorites edit: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsFavoritesEditRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/favorites/edit/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions favorites edit request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsFavoritesEditRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/favorites/edit/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions favorites edit request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsFavoritesEditResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/favorites/edit/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions favorites edit response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsFavoritesEditResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/favorites/edit/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions favorites edit response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandConfigFunctionsFavoritesGetResponseItemSchema = zod.z.union([zod.z.object({
  commit: zod.z.string().nullable().optional(),
  name: zod.z.string(),
  note: zod.z.string(),
  owner: zod.z.string(),
  remote: zod.z.literal("github"),
  repository: zod.z.string()
}).meta({ "variantTitle": "Github" }), zod.z.object({
  commit: zod.z.string().nullable().optional(),
  name: zod.z.string(),
  note: zod.z.string(),
  owner: zod.z.string(),
  remote: zod.z.literal("filesystem"),
  repository: zod.z.string()
}).meta({ "variantTitle": "Filesystem" }), zod.z.object({
  name: zod.z.string(),
  note: zod.z.string(),
  remote: zod.z.literal("mock")
}).meta({ "variantTitle": "Mock" })]).meta({ title: "cli.command.config.functions.favorites.get.ResponseItem" });

// src/viewer/command/config/functions/favorites/get.ts
function configFunctionsFavoritesGetExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/favorites/get" }), zod.z.union([CliErrorSchema, CliCommandConfigFunctionsFavoritesGetResponseItemSchema]));
}
function configFunctionsFavoritesGetExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/favorites/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function configFunctionsFavoritesGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/favorites/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions favorites get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsFavoritesGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/favorites/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions favorites get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsFavoritesGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/favorites/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions favorites get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsFavoritesGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/favorites/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions favorites get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandConfigFunctionsGetResponseSchema = zod.z.object({
  favorites: zod.z.array(CliCommandConfigFunctionsFavoritesGetResponseItemSchema).nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.config.functions.get.Response" });

// src/viewer/command/config/functions/get.ts
async function configFunctionsGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/get" }), zod.z.union([CliErrorSchema, CliCommandConfigFunctionsGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions get: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions get: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesFavoritesAddExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/functions/profiles/favorites/add" }), zod.z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles favorites add: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesFavoritesAddExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/functions/profiles/favorites/add" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles favorites add: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesFavoritesAddRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/profiles/favorites/add/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles favorites add request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesFavoritesAddRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/profiles/favorites/add/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles favorites add request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesFavoritesAddResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/profiles/favorites/add/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles favorites add response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesFavoritesAddResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/profiles/favorites/add/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles favorites add response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesFavoritesDelExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/functions/profiles/favorites/del" }), zod.z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles favorites del: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesFavoritesDelExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/functions/profiles/favorites/del" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles favorites del: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesFavoritesDelRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/profiles/favorites/del/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles favorites del request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesFavoritesDelRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/profiles/favorites/del/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles favorites del request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesFavoritesDelResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/profiles/favorites/del/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles favorites del response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesFavoritesDelResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/profiles/favorites/del/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles favorites del response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesFavoritesEditExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/functions/profiles/favorites/edit" }), zod.z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles favorites edit: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesFavoritesEditExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/functions/profiles/favorites/edit" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles favorites edit: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesFavoritesEditRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/profiles/favorites/edit/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles favorites edit request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesFavoritesEditRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/profiles/favorites/edit/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles favorites edit request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesFavoritesEditResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/profiles/favorites/edit/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles favorites edit response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesFavoritesEditResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/profiles/favorites/edit/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles favorites edit response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandConfigFunctionsProfilesFavoritesGetResponseItemSchema = zod.z.union([zod.z.object({
  commit: zod.z.string().nullable().optional(),
  name: zod.z.string(),
  note: zod.z.string(),
  owner: zod.z.string(),
  remote: zod.z.literal("github"),
  repository: zod.z.string()
}).meta({ "variantTitle": "Github" }), zod.z.object({
  commit: zod.z.string().nullable().optional(),
  name: zod.z.string(),
  note: zod.z.string(),
  owner: zod.z.string(),
  remote: zod.z.literal("filesystem"),
  repository: zod.z.string()
}).meta({ "variantTitle": "Filesystem" }), zod.z.object({
  name: zod.z.string(),
  note: zod.z.string(),
  remote: zod.z.literal("mock")
}).meta({ "variantTitle": "Mock" })]).meta({ title: "cli.command.config.functions.profiles.favorites.get.ResponseItem" });

// src/viewer/command/config/functions/profiles/favorites/get.ts
function configFunctionsProfilesFavoritesGetExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/profiles/favorites/get" }), zod.z.union([CliErrorSchema, CliCommandConfigFunctionsProfilesFavoritesGetResponseItemSchema]));
}
function configFunctionsProfilesFavoritesGetExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/profiles/favorites/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function configFunctionsProfilesFavoritesGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/profiles/favorites/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles favorites get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesFavoritesGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/profiles/favorites/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles favorites get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesFavoritesGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/profiles/favorites/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles favorites get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesFavoritesGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/profiles/favorites/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles favorites get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandConfigFunctionsProfilesGetResponseSchema = zod.z.object({
  favorites: zod.z.array(CliCommandConfigFunctionsProfilesFavoritesGetResponseItemSchema).nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.config.functions.profiles.get.Response" });

// src/viewer/command/config/functions/profiles/get.ts
async function configFunctionsProfilesGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/profiles/get" }), zod.z.union([CliErrorSchema, CliCommandConfigFunctionsProfilesGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles get: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/profiles/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles get: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/profiles/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/profiles/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/profiles/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/profiles/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsFavoritesAddExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/functions/profiles/pairs/favorites/add" }), zod.z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs favorites add: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsFavoritesAddExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/functions/profiles/pairs/favorites/add" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs favorites add: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsFavoritesAddRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/profiles/pairs/favorites/add/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs favorites add request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsFavoritesAddRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/profiles/pairs/favorites/add/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs favorites add request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsFavoritesAddResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/profiles/pairs/favorites/add/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs favorites add response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsFavoritesAddResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/profiles/pairs/favorites/add/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs favorites add response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsFavoritesDelExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/functions/profiles/pairs/favorites/del" }), zod.z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs favorites del: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsFavoritesDelExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/functions/profiles/pairs/favorites/del" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs favorites del: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsFavoritesDelRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/profiles/pairs/favorites/del/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs favorites del request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsFavoritesDelRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/profiles/pairs/favorites/del/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs favorites del request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsFavoritesDelResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/profiles/pairs/favorites/del/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs favorites del response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsFavoritesDelResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/profiles/pairs/favorites/del/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs favorites del response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsFavoritesEditExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/functions/profiles/pairs/favorites/edit" }), zod.z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs favorites edit: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsFavoritesEditExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/functions/profiles/pairs/favorites/edit" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs favorites edit: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsFavoritesEditRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/profiles/pairs/favorites/edit/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs favorites edit request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsFavoritesEditRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/profiles/pairs/favorites/edit/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs favorites edit request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsFavoritesEditResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/profiles/pairs/favorites/edit/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs favorites edit response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsFavoritesEditResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/profiles/pairs/favorites/edit/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs favorites edit response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandConfigFunctionsProfilesPairsFavoritesGetResponseItemSchema = zod.z.object({
  function: RemotePathCommitOptionalSchema,
  name: zod.z.string(),
  note: zod.z.string(),
  profile: RemotePathCommitOptionalSchema
}).meta({ title: "cli.command.config.functions.profiles.pairs.favorites.get.ResponseItem" });

// src/viewer/command/config/functions/profiles/pairs/favorites/get.ts
function configFunctionsProfilesPairsFavoritesGetExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/profiles/pairs/favorites/get" }), zod.z.union([CliErrorSchema, CliCommandConfigFunctionsProfilesPairsFavoritesGetResponseItemSchema]));
}
function configFunctionsProfilesPairsFavoritesGetExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/profiles/pairs/favorites/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function configFunctionsProfilesPairsFavoritesGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/profiles/pairs/favorites/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs favorites get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsFavoritesGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/profiles/pairs/favorites/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs favorites get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsFavoritesGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/profiles/pairs/favorites/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs favorites get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsFavoritesGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/profiles/pairs/favorites/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs favorites get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandConfigFunctionsProfilesPairsGetResponseSchema = zod.z.object({
  favorites: zod.z.array(CliCommandConfigFunctionsProfilesPairsFavoritesGetResponseItemSchema).nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.config.functions.profiles.pairs.get.Response" });

// src/viewer/command/config/functions/profiles/pairs/get.ts
async function configFunctionsProfilesPairsGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/profiles/pairs/get" }), zod.z.union([CliErrorSchema, CliCommandConfigFunctionsProfilesPairsGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs get: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/profiles/pairs/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs get: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/profiles/pairs/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/profiles/pairs/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/profiles/pairs/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/profiles/pairs/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandConfigMcpAddressGetResponseSchema = zod.z.object({
  address: zod.z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.config.mcp.address.get.Response" });

// src/viewer/command/config/mcp/address/get.ts
async function configMcpAddressGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/mcp/address/get" }), zod.z.union([CliErrorSchema, CliCommandConfigMcpAddressGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp address get: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpAddressGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/mcp/address/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp address get: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpAddressGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/mcp/address/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp address get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpAddressGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/mcp/address/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp address get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpAddressGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/mcp/address/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp address get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpAddressGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/mcp/address/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp address get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpAddressSetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/mcp/address/set" }), zod.z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp address set: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpAddressSetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/mcp/address/set" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp address set: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpAddressSetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/mcp/address/set/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp address set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpAddressSetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/mcp/address/set/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp address set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpAddressSetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/mcp/address/set/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp address set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpAddressSetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/mcp/address/set/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp address set response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandConfigMcpGetResponseSchema = zod.z.object({
  address: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  port: zod.z.number().int().min(0).max(65535).nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.config.mcp.get.Response" });

// src/viewer/command/config/mcp/get.ts
async function configMcpGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/mcp/get" }), zod.z.union([CliErrorSchema, CliCommandConfigMcpGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp get: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/mcp/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp get: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/mcp/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/mcp/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/mcp/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/mcp/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandConfigMcpPortGetResponseSchema = zod.z.object({
  port: zod.z.number().int().min(0).max(65535)
}).meta({ title: "cli.command.config.mcp.port.get.Response" });

// src/viewer/command/config/mcp/port/get.ts
async function configMcpPortGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/mcp/port/get" }), zod.z.union([CliErrorSchema, CliCommandConfigMcpPortGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp port get: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpPortGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/mcp/port/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp port get: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpPortGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/mcp/port/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp port get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpPortGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/mcp/port/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp port get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpPortGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/mcp/port/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp port get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpPortGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/mcp/port/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp port get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpPortSetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/mcp/port/set" }), zod.z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp port set: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpPortSetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/mcp/port/set" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp port set: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpPortSetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/mcp/port/set/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp port set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpPortSetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/mcp/port/set/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp port set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpPortSetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/mcp/port/set/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp port set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpPortSetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/mcp/port/set/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp port set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsFavoritesAddExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/swarms/favorites/add" }), zod.z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms favorites add: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsFavoritesAddExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/swarms/favorites/add" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms favorites add: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsFavoritesAddRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/swarms/favorites/add/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms favorites add request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsFavoritesAddRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/swarms/favorites/add/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms favorites add request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsFavoritesAddResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/swarms/favorites/add/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms favorites add response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsFavoritesAddResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/swarms/favorites/add/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms favorites add response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsFavoritesDelExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/swarms/favorites/del" }), zod.z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms favorites del: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsFavoritesDelExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/swarms/favorites/del" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms favorites del: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsFavoritesDelRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/swarms/favorites/del/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms favorites del request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsFavoritesDelRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/swarms/favorites/del/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms favorites del request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsFavoritesDelResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/swarms/favorites/del/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms favorites del response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsFavoritesDelResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/swarms/favorites/del/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms favorites del response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsFavoritesEditExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/swarms/favorites/edit" }), zod.z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms favorites edit: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsFavoritesEditExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/swarms/favorites/edit" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms favorites edit: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsFavoritesEditRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/swarms/favorites/edit/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms favorites edit request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsFavoritesEditRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/swarms/favorites/edit/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms favorites edit request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsFavoritesEditResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/swarms/favorites/edit/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms favorites edit response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsFavoritesEditResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/swarms/favorites/edit/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms favorites edit response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandConfigSwarmsFavoritesGetResponseItemSchema = zod.z.union([zod.z.object({
  commit: zod.z.string().nullable().optional(),
  name: zod.z.string(),
  note: zod.z.string(),
  owner: zod.z.string(),
  remote: zod.z.literal("github"),
  repository: zod.z.string()
}).meta({ "variantTitle": "Github" }), zod.z.object({
  commit: zod.z.string().nullable().optional(),
  name: zod.z.string(),
  note: zod.z.string(),
  owner: zod.z.string(),
  remote: zod.z.literal("filesystem"),
  repository: zod.z.string()
}).meta({ "variantTitle": "Filesystem" }), zod.z.object({
  name: zod.z.string(),
  note: zod.z.string(),
  remote: zod.z.literal("mock")
}).meta({ "variantTitle": "Mock" })]).meta({ title: "cli.command.config.swarms.favorites.get.ResponseItem" });

// src/viewer/command/config/swarms/favorites/get.ts
function configSwarmsFavoritesGetExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/swarms/favorites/get" }), zod.z.union([CliErrorSchema, CliCommandConfigSwarmsFavoritesGetResponseItemSchema]));
}
function configSwarmsFavoritesGetExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/swarms/favorites/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function configSwarmsFavoritesGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/swarms/favorites/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms favorites get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsFavoritesGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/swarms/favorites/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms favorites get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsFavoritesGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/swarms/favorites/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms favorites get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsFavoritesGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/swarms/favorites/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms favorites get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandConfigSwarmsGetResponseSchema = zod.z.object({
  favorites: zod.z.array(CliCommandConfigSwarmsFavoritesGetResponseItemSchema).nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.config.swarms.get.Response" });

// src/viewer/command/config/swarms/get.ts
async function configSwarmsGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/swarms/get" }), zod.z.union([CliErrorSchema, CliCommandConfigSwarmsGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms get: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/swarms/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms get: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/swarms/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/swarms/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/swarms/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/swarms/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandConfigViewerAddressGetResponseSchema = zod.z.object({
  address: zod.z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.config.viewer.address.get.Response" });

// src/viewer/command/config/viewer/address/get.ts
async function configViewerAddressGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/viewer/address/get" }), zod.z.union([CliErrorSchema, CliCommandConfigViewerAddressGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer address get: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerAddressGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/viewer/address/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer address get: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerAddressGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/viewer/address/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer address get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerAddressGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/viewer/address/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer address get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerAddressGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/viewer/address/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer address get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerAddressGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/viewer/address/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer address get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerAddressSetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/viewer/address/set" }), zod.z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer address set: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerAddressSetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/viewer/address/set" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer address set: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerAddressSetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/viewer/address/set/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer address set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerAddressSetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/viewer/address/set/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer address set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerAddressSetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/viewer/address/set/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer address set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerAddressSetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/viewer/address/set/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer address set response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandConfigViewerGetResponseSchema = zod.z.object({
  address: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  port: zod.z.number().int().min(0).max(65535).nullable().meta({ omitempty: true }).optional(),
  secret: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  signature: zod.z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.config.viewer.get.Response" });

// src/viewer/command/config/viewer/get.ts
async function configViewerGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/viewer/get" }), zod.z.union([CliErrorSchema, CliCommandConfigViewerGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer get: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/viewer/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer get: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/viewer/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/viewer/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/viewer/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/viewer/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandConfigViewerPortGetResponseSchema = zod.z.object({
  port: zod.z.number().int().min(0).max(65535)
}).meta({ title: "cli.command.config.viewer.port.get.Response" });

// src/viewer/command/config/viewer/port/get.ts
async function configViewerPortGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/viewer/port/get" }), zod.z.union([CliErrorSchema, CliCommandConfigViewerPortGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer port get: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerPortGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/viewer/port/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer port get: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerPortGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/viewer/port/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer port get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerPortGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/viewer/port/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer port get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerPortGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/viewer/port/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer port get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerPortGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/viewer/port/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer port get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerPortSetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/viewer/port/set" }), zod.z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer port set: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerPortSetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/viewer/port/set" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer port set: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerPortSetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/viewer/port/set/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer port set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerPortSetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/viewer/port/set/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer port set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerPortSetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/viewer/port/set/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer port set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerPortSetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/viewer/port/set/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer port set response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandConfigViewerSecretGetResponseSchema = zod.z.object({
  secret: zod.z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.config.viewer.secret.get.Response" });

// src/viewer/command/config/viewer/secret/get.ts
async function configViewerSecretGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/viewer/secret/get" }), zod.z.union([CliErrorSchema, CliCommandConfigViewerSecretGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer secret get: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerSecretGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/viewer/secret/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer secret get: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerSecretGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/viewer/secret/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer secret get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerSecretGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/viewer/secret/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer secret get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerSecretGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/viewer/secret/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer secret get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerSecretGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/viewer/secret/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer secret get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerSecretSetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/viewer/secret/set" }), zod.z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer secret set: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerSecretSetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/viewer/secret/set" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer secret set: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerSecretSetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/viewer/secret/set/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer secret set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerSecretSetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/viewer/secret/set/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer secret set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerSecretSetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/viewer/secret/set/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer secret set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerSecretSetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/viewer/secret/set/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer secret set response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandConfigViewerSignatureGetResponseSchema = zod.z.object({
  signature: zod.z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.config.viewer.signature.get.Response" });

// src/viewer/command/config/viewer/signature/get.ts
async function configViewerSignatureGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/viewer/signature/get" }), zod.z.union([CliErrorSchema, CliCommandConfigViewerSignatureGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer signature get: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerSignatureGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/viewer/signature/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer signature get: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerSignatureGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/viewer/signature/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer signature get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerSignatureGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/viewer/signature/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer signature get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerSignatureGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/viewer/signature/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer signature get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerSignatureGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/viewer/signature/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer signature get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerSignatureSetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/viewer/signature/set" }), zod.z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer signature set: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerSignatureSetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/viewer/signature/set" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer signature set: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerSignatureSetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/viewer/signature/set/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer signature set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerSignatureSetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/viewer/signature/set/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer signature set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerSignatureSetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/viewer/signature/set/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer signature set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerSignatureSetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/viewer/signature/set/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer signature set response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandDbQueryColumnSchema = zod.z.object({
  name: zod.z.string(),
  type: zod.z.string()
}).describe('One result column. `r#type` is the Postgres `pg_type.typname`\n(e.g. `"int8"`, `"text"`, `"jsonb"`, `"timestamptz"`). Callers\nneeding precision/scale/array-element-type can inspect the\nname; richer typeinfo is deferred.').meta({ title: "cli.command.db.query.Column" });

// src/cli/command/db/query/response.ts
var CliCommandDbQueryResponseSchema = zod.z.object({
  columns: zod.z.array(CliCommandDbQueryColumnSchema).describe("Result columns in select-list order. Empty for no-row\nstatements (SET / LISTEN / DO)."),
  command_tag: zod.z.string().describe('Wire-protocol-style command tag \u2014 e.g. `"SELECT 5"`,\n`"SHOW 1"`, `"EXPLAIN 12"`, `"SET 0"`. Synthesized from\n`{leading_keyword} {row_count}` since sqlx 0.8 doesn\'t\nexpose the wire-protocol `CommandComplete` tag; close\nenough for telemetry / display.'),
  rows: zod.z.array(zod.z.array(JsonValueSchema)).describe("One inner Vec per row, length matches `columns.len()`.\nEach cell is a `serde_json::Value` produced by the CLI's\nper-cell `pg_value_to_json` decoder. Common type\nencodings: text/uuid/timestamps as JSON String, numeric\nas String (preserving precision), bytea as base64 String,\njson/jsonb passthrough as Value, arrays recurse. Empty\nwhen the statement returns no rows (no-row command OR a\nSELECT with no matches)."),
  truncated: zod.z.boolean().describe('Always `false` in the current design \u2014 over-budget\nresponses return `TokenBudgetExceeded` rather than a\npartial result. Reserved on the wire so a future "soft\ntruncation" mode can be added without a shape break.')
}).describe("Unary response from `db query`.").meta({ title: "cli.command.db.query.Response" });

// src/viewer/command/db/query.ts
async function dbQueryExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "db/query" }), zod.z.union([CliErrorSchema, CliCommandDbQueryResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db query: cli produced no output before the end marker");
  }
  return first;
}
async function dbQueryExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "db/query" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db query: cli produced no output before the end marker");
  }
  return first;
}
async function dbQueryRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "db/query/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db query request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbQueryRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "db/query/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db query request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbQueryResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "db/query/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db query response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbQueryResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "db/query/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db query response_schema: cli produced no output before the end marker");
  }
  return first;
}
var FunctionsExpressionTaskOutputSchema = zod.z.union([zod.z.number().min(-34028234663852886e22).max(34028234663852886e22).describe("A single scalar score.").meta({ "variantTitle": "Scalar" }), zod.z.array(zod.z.number().min(-34028234663852886e22).max(34028234663852886e22)).describe("A vector of scores.").meta({ "variantTitle": "Vector" }), zod.z.array(zod.z.array(zod.z.number().min(-34028234663852886e22).max(34028234663852886e22))).describe("Multiple vectors of scores (from mapped tasks).").meta({ "variantTitle": "Vectors" }), zod.z.object({
  error: JsonValueSchema
}).describe("An error occurred during execution.").meta({ "variantTitle": "Err" })]).describe("Owned task output variants.").meta({ title: "functions.expression.TaskOutput" });

// src/functions/executions/response/output.ts
var FunctionsExecutionsResponseOutputSchema = zod.z.object({
  output: FunctionsExpressionTaskOutputSchema
}).describe("Wrapper for function execution output, distinguishing between\na null output value and a missing output.").meta({ title: "functions.executions.response.Output" });
var FunctionsExecutionsResponseStreamingObjectSchema = zod.z.enum(["scalar.function.execution.chunk", "vector.function.execution.chunk"]).meta({ title: "functions.executions.response.streaming.Object" });
var FunctionsExecutionsResponseStreamingReasoningSummaryChunkSchema = zod.z.object({
  agent_full_id: zod.z.string().describe("WF-level id: concatenation of the primary agent's id with all\nfallback ids (see `InlineAgentWithFallbacks::full_id`). Same\nfor every slot in the same WF request."),
  agent_id: zod.z.string().describe("Leaf agent id of the slot that produced this chunk. For the\nprimary attempt this is the primary agent's id; on fallback it\nis the fallback agent's id. Same on every chunk of a slot."),
  agent_instance_hierarchy: zod.z.string().describe("Full agent instance hierarchy for this completion's slot \u2014\n`{ctx lineage}/{agent_full_id}-{response_id}`, or the fixed\ncontinuation value on resume. Same on every chunk of a slot."),
  agent_remote: RemotePathSchema.nullable().describe("`RemotePath` the WF was fetched from. `None` when the WF was\nsupplied inline. Same for every slot in the same WF request.").meta({ omitempty: true }).optional(),
  continuation: zod.z.string().nullable().describe("Continuation state for multi-turn conversations (only present in the final chunk).").meta({ omitempty: true }).optional(),
  created: zod.z.number().int().min(0).max(18446744073709552e3),
  error: ErrorResponseErrorSchema.nullable().meta({ omitempty: true }).optional(),
  id: zod.z.string(),
  messages: zod.z.array(AgentCompletionsResponseStreamingMessageChunkSchema),
  messages_queued: zod.z.boolean().nullable().describe("`true` when the MCP proxy holds queued messages that were not\ndelivered to the agent via a tool response on this turn. Only\nset when `continuation` is also set \u2014 the caller acts on it by\nissuing the continuation. Absent when nothing is queued, when\nthere is no continuation to act on, or when the peek failed\n(the failure is surfaced via `error`).").meta({ omitempty: true }).optional(),
  object: AgentCompletionsResponseStreamingObjectSchema.describe('The object type (always "agent.completion.chunk").'),
  upstream: AgentUpstreamSchema.describe("Upstream provider"),
  usage: AgentCompletionsResponseUsageSchema.nullable().describe("Token usage (only present in the final chunk).").meta({ omitempty: true }).optional()
}).describe("A chunk of a streaming agent completion response.\n\nMultiple chunks are received via Server-Sent Events and can be\naccumulated into a complete [`AgentCompletion`](response::unary::AgentCompletion)\nusing the [`push`](Self::push) method.").meta({ title: "functions.executions.response.streaming.ReasoningSummaryChunk" });
var FunctionsExecutionsResponseStreamingFunctionExecutionTaskChunkSchema = zod.z.object({
  created: zod.z.number().int().min(0).max(18446744073709552e3),
  error: ErrorResponseErrorSchema.nullable().meta({ omitempty: true }).optional(),
  function: RemotePathSchema.nullable().optional(),
  id: zod.z.string(),
  index: zod.z.number().int().min(0).max(18446744073709552e3),
  object: FunctionsExecutionsResponseStreamingObjectSchema,
  output: FunctionsExecutionsResponseOutputSchema.nullable().meta({ omitempty: true }).optional(),
  profile: RemotePathSchema.nullable().optional(),
  reasoning: FunctionsExecutionsResponseStreamingReasoningSummaryChunkSchema.nullable().meta({ omitempty: true }).optional(),
  retry_token: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  split_index: zod.z.number().int().min(0).max(18446744073709552e3).nullable().meta({ omitempty: true }).optional(),
  swiss_pool_index: zod.z.number().int().min(0).max(18446744073709552e3).nullable().meta({ omitempty: true }).optional(),
  swiss_round: zod.z.number().int().min(0).max(18446744073709552e3).nullable().meta({ omitempty: true }).optional(),
  task_index: zod.z.number().int().min(0).max(18446744073709552e3),
  task_path: zod.z.array(zod.z.number().int().min(0).max(18446744073709552e3)),
  tasks: zod.z.array(zod.z.lazy(() => FunctionsExecutionsResponseStreamingTaskChunkSchema)),
  tasks_errors: zod.z.boolean().nullable().meta({ omitempty: true }).optional(),
  usage: AgentCompletionsResponseUsageSchema.nullable().meta({ omitempty: true }).optional()
}).meta({ title: "functions.executions.response.streaming.FunctionExecutionTaskChunk" });
var VectorCompletionsResponseStreamingAgentCompletionChunkSchema = zod.z.object({
  agent_full_id: zod.z.string().describe("WF-level id: concatenation of the primary agent's id with all\nfallback ids (see `InlineAgentWithFallbacks::full_id`). Same\nfor every slot in the same WF request."),
  agent_id: zod.z.string().describe("Leaf agent id of the slot that produced this chunk. For the\nprimary attempt this is the primary agent's id; on fallback it\nis the fallback agent's id. Same on every chunk of a slot."),
  agent_instance_hierarchy: zod.z.string().describe("Full agent instance hierarchy for this completion's slot \u2014\n`{ctx lineage}/{agent_full_id}-{response_id}`, or the fixed\ncontinuation value on resume. Same on every chunk of a slot."),
  agent_remote: RemotePathSchema.nullable().describe("`RemotePath` the WF was fetched from. `None` when the WF was\nsupplied inline. Same for every slot in the same WF request.").meta({ omitempty: true }).optional(),
  continuation: zod.z.string().nullable().describe("Continuation state for multi-turn conversations (only present in the final chunk).").meta({ omitempty: true }).optional(),
  created: zod.z.number().int().min(0).max(18446744073709552e3),
  error: ErrorResponseErrorSchema.nullable().describe("Error details if this completion failed.").meta({ omitempty: true }).optional(),
  id: zod.z.string(),
  index: zod.z.number().int().min(0).max(18446744073709552e3).describe("Index used to correlate chunks from the same completion."),
  messages: zod.z.array(AgentCompletionsResponseStreamingMessageChunkSchema),
  messages_queued: zod.z.boolean().nullable().describe("`true` when the MCP proxy holds queued messages that were not\ndelivered to the agent via a tool response on this turn. Only\nset when `continuation` is also set \u2014 the caller acts on it by\nissuing the continuation. Absent when nothing is queued, when\nthere is no continuation to act on, or when the peek failed\n(the failure is surfaced via `error`).").meta({ omitempty: true }).optional(),
  object: AgentCompletionsResponseStreamingObjectSchema.describe('The object type (always "agent.completion.chunk").'),
  upstream: AgentUpstreamSchema.describe("Upstream provider"),
  usage: AgentCompletionsResponseUsageSchema.nullable().describe("Token usage (only present in the final chunk).").meta({ omitempty: true }).optional()
}).describe("A streaming agent completion chunk from a single agent within a vector completion.\n\nThe `index` field is used to correlate chunks belonging to the same\nunderlying completion when accumulating via [`push`](Self::push).").meta({ title: "vector.completions.response.streaming.AgentCompletionChunk" });
var VectorCompletionsResponseStreamingObjectSchema = zod.z.literal("vector.completion.chunk").describe("A streaming vector completion chunk.").meta({ title: "vector.completions.response.streaming.Object" });
var VectorCompletionsResponseVoteSchema = zod.z.object({
  agent_full_id: zod.z.string().describe("WF-level id of the agent that produced this vote \u2014 concatenation\nof the primary agent's id with all fallback ids (see\n`InlineAgentWithFallbacks::full_id`). Same for every slot in the\nsame WF request and deterministic across api processes."),
  agent_id: zod.z.string().describe("Leaf agent id of the slot that produced this vote (matches the\n`agent_id` on the corresponding\n[`super::super::super::super::agent::completions::response::unary::AgentCompletion`]).\nWhen fallbacks fired, this is the fallback's id rather than the\nprimary's."),
  flat_swarm_index: zod.z.number().int().min(0).max(18446744073709552e3).describe("Flattened index accounting for agent counts in the swarm."),
  from_cache: zod.z.boolean().nullable().describe("If true, this vote was retrieved from cache rather than generated fresh.").meta({ omitempty: true }).optional(),
  prompt_id: zod.z.string().describe("Content hash of the request messages (for caching/deduplication)."),
  responses_ids: zod.z.array(zod.z.string()).describe("Content hashes of each response option in the request."),
  retry: zod.z.boolean().nullable().describe("If true, this vote was reused from a previous request via the `retry`\nparameter. All fields reflect the original request's values.").meta({ omitempty: true }).optional(),
  swarm_index: zod.z.number().int().min(0).max(18446744073709552e3).describe("Index of the agent configuration within the swarm."),
  vote: zod.z.array(zod.z.number().min(-34028234663852886e22).max(34028234663852886e22)).describe("The vote distribution. Each index corresponds to a response from the\nrequest. Typically one element is 1.0 (selected) and the rest are 0.0."),
  weight: zod.z.number().min(-34028234663852886e22).max(34028234663852886e22).describe("The weight applied to this vote when computing final scores.")
}).describe("A single LLM's vote in a vector completion.\n\nEach LLM in the swarm produces a vote indicating which response(s) it\nselected. Votes are weighted according to the profile and combined to\nproduce the final scores.\n\n# Vote Format\n\nThe `vote` field is a vector of decimals corresponding to the responses\nin the request. Typically one element is 1.0 and the rest are 0.0 (discrete\nselection), but when `top_logprobs` is used, votes may be probability\ndistributions.").meta({ title: "vector.completions.response.Vote" });

// src/functions/executions/response/streaming/vectorCompletionTaskChunk.ts
var FunctionsExecutionsResponseStreamingVectorCompletionTaskChunkSchema = zod.z.object({
  completions: zod.z.array(VectorCompletionsResponseStreamingAgentCompletionChunkSchema).describe("Incremental agent completion chunks from each agent."),
  created: zod.z.number().int().min(0).max(18446744073709552e3).describe("Unix timestamp when the completion was created."),
  error: ErrorResponseErrorSchema.nullable().meta({ omitempty: true }).optional(),
  id: zod.z.string().describe("Unique identifier for this vector completion."),
  index: zod.z.number().int().min(0).max(18446744073709552e3),
  object: VectorCompletionsResponseStreamingObjectSchema.describe('Object type identifier (`"vector.completion.chunk"`).'),
  scores: zod.z.array(zod.z.number().min(-34028234663852886e22).max(34028234663852886e22)).describe("Current weighted scores. Updated as new votes arrive."),
  swarm: zod.z.string().describe("ID of the swarm used for this completion."),
  task_index: zod.z.number().int().min(0).max(18446744073709552e3),
  task_path: zod.z.array(zod.z.number().int().min(0).max(18446744073709552e3)),
  usage: AgentCompletionsResponseUsageSchema.nullable().describe("Aggregated usage statistics. Typically present only in the final chunk.").meta({ omitempty: true }).optional(),
  votes: zod.z.array(VectorCompletionsResponseVoteSchema).describe("Votes received so far. New votes are appended in subsequent chunks."),
  weights: zod.z.array(zod.z.number().min(-34028234663852886e22).max(34028234663852886e22)).describe("Current weight distribution across responses. Updated as new votes arrive.")
}).describe("A chunk in a streaming vector completion response.\n\nEach chunk contains incremental updates to the completion. Use the\n[`push`](Self::push) method to accumulate chunks into a complete response.").meta({ title: "functions.executions.response.streaming.VectorCompletionTaskChunk" });

// src/functions/executions/response/streaming/taskChunk.ts
var FunctionsExecutionsResponseStreamingTaskChunkSchema = zod.z.union([zod.z.lazy(() => FunctionsExecutionsResponseStreamingFunctionExecutionTaskChunkSchema).meta({ "title": "functions.executions.response.streaming.FunctionExecutionTaskChunk", "variantTitle": "FunctionExecution" }), FunctionsExecutionsResponseStreamingVectorCompletionTaskChunkSchema.meta({ "title": "functions.executions.response.streaming.VectorCompletionTaskChunk", "variantTitle": "VectorCompletion" })]).meta({ title: "functions.executions.response.streaming.TaskChunk" });

// src/functions/executions/response/streaming/functionExecutionChunk.ts
var FunctionsExecutionsResponseStreamingFunctionExecutionChunkSchema = zod.z.object({
  created: zod.z.number().int().min(0).max(18446744073709552e3),
  error: ErrorResponseErrorSchema.nullable().meta({ omitempty: true }).optional(),
  function: RemotePathSchema.nullable().optional(),
  id: zod.z.string(),
  object: FunctionsExecutionsResponseStreamingObjectSchema,
  output: FunctionsExecutionsResponseOutputSchema.nullable().meta({ omitempty: true }).optional(),
  profile: RemotePathSchema.nullable().optional(),
  reasoning: FunctionsExecutionsResponseStreamingReasoningSummaryChunkSchema.nullable().meta({ omitempty: true }).optional(),
  retry_token: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  tasks: zod.z.array(FunctionsExecutionsResponseStreamingTaskChunkSchema),
  tasks_errors: zod.z.boolean().nullable().meta({ omitempty: true }).optional(),
  usage: AgentCompletionsResponseUsageSchema.nullable().meta({ omitempty: true }).optional()
}).meta({ title: "functions.executions.response.streaming.FunctionExecutionChunk" });

// src/cli/command/functions/execute/standard/responseItem.ts
var CliCommandFunctionsExecuteStandardResponseItemSchema = zod.z.union([FunctionsExecutionsResponseStreamingFunctionExecutionChunkSchema.meta({ "title": "functions.executions.response.streaming.FunctionExecutionChunk", "variantTitle": "Chunk" }), zod.z.string().meta({ "variantTitle": "Id" })]).meta({ title: "cli.command.functions.execute.standard.ResponseItem" });

// src/viewer/command/functions/execute/standard.ts
function functionsExecuteStandardExecuteStreaming(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, dangerous_advanced: { ...request.dangerous_advanced ?? {}, stream: true }, path_type: "functions/execute/standard" }), zod.z.union([CliErrorSchema, CliCommandFunctionsExecuteStandardResponseItemSchema]));
}
function functionsExecuteStandardExecuteStreamingJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, dangerous_advanced: { ...request.dangerous_advanced ?? {}, stream: true }, path_type: "functions/execute/standard" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function functionsExecuteStandardExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, dangerous_advanced: request.dangerous_advanced ? { ...request.dangerous_advanced, stream: void 0 } : request.dangerous_advanced, path_type: "functions/execute/standard" }), zod.z.union([CliErrorSchema, zod.z.string()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions execute standard: cli produced no output before the end marker");
  }
  return first;
}
async function functionsExecuteStandardExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, dangerous_advanced: request.dangerous_advanced ? { ...request.dangerous_advanced, stream: void 0 } : request.dangerous_advanced, path_type: "functions/execute/standard" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions execute standard: cli produced no output before the end marker");
  }
  return first;
}
async function functionsExecuteStandardRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/execute/standard/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions execute standard request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsExecuteStandardRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/execute/standard/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions execute standard request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsExecuteStandardResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/execute/standard/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions execute standard response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsExecuteStandardResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/execute/standard/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions execute standard response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandFunctionsExecuteSwissSystemResponseItemSchema = zod.z.union([FunctionsExecutionsResponseStreamingFunctionExecutionChunkSchema.meta({ "title": "functions.executions.response.streaming.FunctionExecutionChunk", "variantTitle": "Chunk" }), zod.z.string().meta({ "variantTitle": "Id" })]).meta({ title: "cli.command.functions.execute.swiss_system.ResponseItem" });

// src/viewer/command/functions/execute/swiss_system.ts
function functionsExecuteSwissSystemExecuteStreaming(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, dangerous_advanced: { ...request.dangerous_advanced ?? {}, stream: true }, path_type: "functions/execute/swiss_system" }), zod.z.union([CliErrorSchema, CliCommandFunctionsExecuteSwissSystemResponseItemSchema]));
}
function functionsExecuteSwissSystemExecuteStreamingJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, dangerous_advanced: { ...request.dangerous_advanced ?? {}, stream: true }, path_type: "functions/execute/swiss_system" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function functionsExecuteSwissSystemExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, dangerous_advanced: request.dangerous_advanced ? { ...request.dangerous_advanced, stream: void 0 } : request.dangerous_advanced, path_type: "functions/execute/swiss_system" }), zod.z.union([CliErrorSchema, zod.z.string()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions execute swiss_system: cli produced no output before the end marker");
  }
  return first;
}
async function functionsExecuteSwissSystemExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, dangerous_advanced: request.dangerous_advanced ? { ...request.dangerous_advanced, stream: void 0 } : request.dangerous_advanced, path_type: "functions/execute/swiss_system" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions execute swiss_system: cli produced no output before the end marker");
  }
  return first;
}
async function functionsExecuteSwissSystemRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/execute/swiss_system/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions execute swiss_system request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsExecuteSwissSystemRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/execute/swiss_system/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions execute swiss_system request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsExecuteSwissSystemResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/execute/swiss_system/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions execute swiss_system response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsExecuteSwissSystemResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/execute/swiss_system/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions execute swiss_system response_schema: cli produced no output before the end marker");
  }
  return first;
}
var FunctionsGetFunctionResponseSchema = RemotePathSchema.and(zod.z.object({})).meta({ title: "functions.GetFunctionResponse" });

// src/viewer/command/functions/get.ts
async function functionsGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/get" }), zod.z.union([CliErrorSchema, FunctionsGetFunctionResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions get: cli produced no output before the end marker");
  }
  return first;
}
async function functionsGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions get: cli produced no output before the end marker");
  }
  return first;
}
async function functionsGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandFunctionsListResponseFavoriteSchema = zod.z.union([zod.z.object({
  commit: zod.z.string().nullable().optional(),
  name: zod.z.string(),
  note: zod.z.string(),
  owner: zod.z.string(),
  remote: zod.z.literal("github"),
  repository: zod.z.string()
}).meta({ "variantTitle": "Github" }), zod.z.object({
  commit: zod.z.string().nullable().optional(),
  name: zod.z.string(),
  note: zod.z.string(),
  owner: zod.z.string(),
  remote: zod.z.literal("filesystem"),
  repository: zod.z.string()
}).meta({ "variantTitle": "Filesystem" }), zod.z.object({
  name: zod.z.string(),
  note: zod.z.string(),
  remote: zod.z.literal("mock")
}).meta({ "variantTitle": "Mock" })]).meta({ title: "cli.command.functions.list.ResponseFavorite" });

// src/cli/command/functions/list/responseItem.ts
var CliCommandFunctionsListResponseItemSchema = zod.z.union([CliCommandFunctionsListResponseFavoriteSchema.meta({ "title": "cli.command.functions.list.ResponseFavorite", "variantTitle": "Favorite" }), RemotePathSchema.meta({ "title": "RemotePath", "variantTitle": "Item" })]).meta({ title: "cli.command.functions.list.ResponseItem" });

// src/viewer/command/functions/list.ts
function functionsListExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/list" }), zod.z.union([CliErrorSchema, CliCommandFunctionsListResponseItemSchema]));
}
function functionsListExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/list" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function functionsListRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/list/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsListRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/list/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsListResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/list/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions list response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsListResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/list/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions list response_schema: cli produced no output before the end marker");
  }
  return first;
}
var FunctionsProfilesGetProfileResponseSchema = RemotePathSchema.and(zod.z.object({})).meta({ title: "functions.profiles.GetProfileResponse" });

// src/viewer/command/functions/profiles/get.ts
async function functionsProfilesGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/profiles/get" }), zod.z.union([CliErrorSchema, FunctionsProfilesGetProfileResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles get: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/profiles/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles get: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/profiles/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/profiles/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/profiles/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/profiles/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandFunctionsProfilesListResponseFavoriteSchema = zod.z.union([zod.z.object({
  commit: zod.z.string().nullable().optional(),
  name: zod.z.string(),
  note: zod.z.string(),
  owner: zod.z.string(),
  remote: zod.z.literal("github"),
  repository: zod.z.string()
}).meta({ "variantTitle": "Github" }), zod.z.object({
  commit: zod.z.string().nullable().optional(),
  name: zod.z.string(),
  note: zod.z.string(),
  owner: zod.z.string(),
  remote: zod.z.literal("filesystem"),
  repository: zod.z.string()
}).meta({ "variantTitle": "Filesystem" }), zod.z.object({
  name: zod.z.string(),
  note: zod.z.string(),
  remote: zod.z.literal("mock")
}).meta({ "variantTitle": "Mock" })]).meta({ title: "cli.command.functions.profiles.list.ResponseFavorite" });

// src/cli/command/functions/profiles/list/responseItem.ts
var CliCommandFunctionsProfilesListResponseItemSchema = zod.z.union([CliCommandFunctionsProfilesListResponseFavoriteSchema.meta({ "title": "cli.command.functions.profiles.list.ResponseFavorite", "variantTitle": "Favorite" }), RemotePathSchema.meta({ "title": "RemotePath", "variantTitle": "Item" })]).meta({ title: "cli.command.functions.profiles.list.ResponseItem" });

// src/viewer/command/functions/profiles/list.ts
function functionsProfilesListExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/profiles/list" }), zod.z.union([CliErrorSchema, CliCommandFunctionsProfilesListResponseItemSchema]));
}
function functionsProfilesListExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/profiles/list" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function functionsProfilesListRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/profiles/list/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesListRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/profiles/list/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesListResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/profiles/list/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles list response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesListResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/profiles/list/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles list response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandFunctionsProfilesPublishResponseSchema = zod.z.object({
  sha: zod.z.string()
}).meta({ title: "cli.command.functions.profiles.publish.Response" });

// src/viewer/command/functions/profiles/publish.ts
async function functionsProfilesPublishExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/profiles/publish" }), zod.z.union([CliErrorSchema, CliCommandFunctionsProfilesPublishResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles publish: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesPublishExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/profiles/publish" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles publish: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesPublishRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/profiles/publish/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles publish request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesPublishRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/profiles/publish/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles publish request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesPublishResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/profiles/publish/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles publish response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesPublishResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/profiles/publish/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles publish response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandFunctionsPublishResponseSchema = zod.z.object({
  sha: zod.z.string()
}).meta({ title: "cli.command.functions.publish.Response" });

// src/viewer/command/functions/publish.ts
async function functionsPublishExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/publish" }), zod.z.union([CliErrorSchema, CliCommandFunctionsPublishResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions publish: cli produced no output before the end marker");
  }
  return first;
}
async function functionsPublishExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/publish" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions publish: cli produced no output before the end marker");
  }
  return first;
}
async function functionsPublishRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/publish/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions publish request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsPublishRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/publish/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions publish request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsPublishResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/publish/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions publish response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsPublishResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/publish/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions publish response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandMcpKillResponseSchema = zod.z.object({
  killed: zod.z.number().int().min(0).max(4294967295)
}).meta({ title: "cli.command.mcp.kill.Response" });

// src/viewer/command/mcp/kill.ts
async function mcpKillExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "mcp/kill" }), zod.z.union([CliErrorSchema, CliCommandMcpKillResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp kill: cli produced no output before the end marker");
  }
  return first;
}
async function mcpKillExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "mcp/kill" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp kill: cli produced no output before the end marker");
  }
  return first;
}
async function mcpKillRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "mcp/kill/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp kill request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpKillRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "mcp/kill/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp kill request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpKillResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "mcp/kill/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp kill response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpKillResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "mcp/kill/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp kill response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandMcpSpawnResponseSchema = zod.z.object({
  listening: zod.z.string()
}).meta({ title: "cli.command.mcp.spawn.Response" });

// src/viewer/command/mcp/spawn.ts
async function mcpSpawnExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "mcp/spawn" }), zod.z.union([CliErrorSchema, CliCommandMcpSpawnResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp spawn: cli produced no output before the end marker");
  }
  return first;
}
async function mcpSpawnExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "mcp/spawn" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp spawn: cli produced no output before the end marker");
  }
  return first;
}
async function mcpSpawnRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "mcp/spawn/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp spawn request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpSpawnRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "mcp/spawn/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp spawn request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpSpawnResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "mcp/spawn/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp spawn response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpSpawnResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "mcp/spawn/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp spawn response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandPluginsGetResponseBinariesSchema = zod.z.object({
  linux_aarch64: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  linux_x86_64: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  macos_aarch64: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  macos_x86_64: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  windows_aarch64: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  windows_x86_64: zod.z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.plugins.get.ResponseBinaries" });
var CliCommandPluginsGetResponseMcpServerSchema = zod.z.object({
  authorization: zod.z.boolean(),
  name: zod.z.string(),
  url: zod.z.string()
}).meta({ title: "cli.command.plugins.get.ResponseMcpServer" });
var CliCommandPluginsGetResponseHttpMethodSchema = zod.z.enum(["GET", "POST", "PUT", "PATCH", "DELETE"]).meta({ title: "cli.command.plugins.get.ResponseHttpMethod" });

// src/cli/command/plugins/get/responseViewerRoute.ts
var CliCommandPluginsGetResponseViewerRouteSchema = zod.z.object({
  method: CliCommandPluginsGetResponseHttpMethodSchema,
  path: zod.z.string(),
  type: zod.z.string()
}).meta({ title: "cli.command.plugins.get.ResponseViewerRoute" });

// src/cli/command/plugins/get/responseManifest.ts
var CliCommandPluginsGetResponseManifestSchema = zod.z.object({
  author: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  binaries: CliCommandPluginsGetResponseBinariesSchema,
  description: zod.z.string(),
  homepage: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  license: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  mcp_servers: zod.z.array(CliCommandPluginsGetResponseMcpServerSchema),
  mobile_ready: zod.z.boolean(),
  name: zod.z.string(),
  owner: zod.z.string(),
  source: zod.z.string(),
  version: zod.z.string(),
  viewer_routes: zod.z.array(CliCommandPluginsGetResponseViewerRouteSchema),
  viewer_url: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  viewer_zip: zod.z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.plugins.get.ResponseManifest" });

// src/viewer/command/plugins/get.ts
async function pluginsGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "plugins/get" }), zod.z.union([CliErrorSchema, CliCommandPluginsGetResponseManifestSchema.nullable()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins get: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "plugins/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins get: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "plugins/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "plugins/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "plugins/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "plugins/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandPluginsInstallFilesystemResponseSchema = zod.z.object({
  instructions: zod.z.string()
}).meta({ title: "cli.command.plugins.install.filesystem.Response" });

// src/viewer/command/plugins/install/filesystem.ts
async function pluginsInstallFilesystemExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "plugins/install/filesystem" }), zod.z.union([CliErrorSchema, CliCommandPluginsInstallFilesystemResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins install filesystem: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsInstallFilesystemExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "plugins/install/filesystem" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins install filesystem: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsInstallFilesystemRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "plugins/install/filesystem/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins install filesystem request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsInstallFilesystemRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "plugins/install/filesystem/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins install filesystem request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsInstallFilesystemResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "plugins/install/filesystem/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins install filesystem response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsInstallFilesystemResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "plugins/install/filesystem/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins install filesystem response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandPluginsInstallGithubResponseSchema = zod.z.object({
  installed: zod.z.boolean()
}).meta({ title: "cli.command.plugins.install.github.Response" });

// src/viewer/command/plugins/install/github.ts
async function pluginsInstallGithubExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "plugins/install/github" }), zod.z.union([CliErrorSchema, CliCommandPluginsInstallGithubResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins install github: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsInstallGithubExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "plugins/install/github" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins install github: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsInstallGithubRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "plugins/install/github/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins install github request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsInstallGithubRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "plugins/install/github/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins install github request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsInstallGithubResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "plugins/install/github/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins install github response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsInstallGithubResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "plugins/install/github/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins install github response_schema: cli produced no output before the end marker");
  }
  return first;
}
function pluginsListExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "plugins/list" }), zod.z.union([CliErrorSchema, CliCommandPluginsGetResponseManifestSchema]));
}
function pluginsListExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "plugins/list" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function pluginsListRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "plugins/list/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsListRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "plugins/list/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsListResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "plugins/list/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins list response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsListResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "plugins/list/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins list response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandPluginsRunMcpTypeSchema = zod.z.literal("mcp").describe('Single-variant discriminator for [`Mcp`]\'s `type` field. Always\n`"mcp"` on the wire.').meta({ title: "cli.command.plugins.run.McpType" });

// src/cli/command/plugins/run/mcp.ts
var CliCommandPluginsRunMcpSchema = zod.z.object({
  type: CliCommandPluginsRunMcpTypeSchema,
  url: zod.z.string()
}).describe('Plugin announces a running MCP server URL. The host routes this\nthrough the standard plugin-notification pipeline and dials the\nURL the same way it would for an entry in the plugin\'s manifest\n`mcp_servers` \u2014 runtime announcements are functionally identical\nto manifest-time declarations.\n\nThe constant `type:"mcp"` discriminator disambiguates this\nvariant from the rest of the untagged [`ResponseItem`] /\n[`crate::cli::plugins::Output`] catch-all, mirroring the\n`type:"error"` discriminator on [`crate::cli::Error`].').meta({ title: "cli.command.plugins.run.Mcp" });

// src/cli/command/plugins/run/responseItem.ts
var CliCommandPluginsRunResponseItemSchema = zod.z.union([CliCommandPluginsRunMcpSchema.meta({ "title": "cli.command.plugins.run.Mcp", "variantTitle": "Mcp" }), CliErrorSchema.meta({ "title": "cli.Error", "variantTitle": "Error" }), JsonValueSchema.meta({ "variantTitle": "Notification" })]).meta({ title: "cli.command.plugins.run.ResponseItem" });

// src/viewer/command/plugins/run.ts
function pluginsRunExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "plugins/run" }), zod.z.union([CliErrorSchema, CliCommandPluginsRunResponseItemSchema]));
}
function pluginsRunExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "plugins/run" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function pluginsRunRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "plugins/run/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins run request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsRunRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "plugins/run/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins run request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsRunResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "plugins/run/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins run response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsRunResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "plugins/run/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins run response_schema: cli produced no output before the end marker");
  }
  return first;
}
var SwarmGetSwarmResponseSchema = RemotePathSchema.and(zod.z.object({})).describe("Response containing a single Swarm.").meta({ title: "swarm.GetSwarmResponse" });

// src/viewer/command/swarms/get.ts
async function swarmsGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "swarms/get" }), zod.z.union([CliErrorSchema, SwarmGetSwarmResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms get: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "swarms/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms get: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "swarms/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "swarms/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "swarms/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "swarms/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandSwarmsListResponseFavoriteSchema = zod.z.union([zod.z.object({
  commit: zod.z.string().nullable().optional(),
  name: zod.z.string(),
  note: zod.z.string(),
  owner: zod.z.string(),
  remote: zod.z.literal("github"),
  repository: zod.z.string()
}).meta({ "variantTitle": "Github" }), zod.z.object({
  commit: zod.z.string().nullable().optional(),
  name: zod.z.string(),
  note: zod.z.string(),
  owner: zod.z.string(),
  remote: zod.z.literal("filesystem"),
  repository: zod.z.string()
}).meta({ "variantTitle": "Filesystem" }), zod.z.object({
  name: zod.z.string(),
  note: zod.z.string(),
  remote: zod.z.literal("mock")
}).meta({ "variantTitle": "Mock" })]).meta({ title: "cli.command.swarms.list.ResponseFavorite" });

// src/cli/command/swarms/list/responseItem.ts
var CliCommandSwarmsListResponseItemSchema = zod.z.union([CliCommandSwarmsListResponseFavoriteSchema.meta({ "title": "cli.command.swarms.list.ResponseFavorite", "variantTitle": "Favorite" }), RemotePathSchema.meta({ "title": "RemotePath", "variantTitle": "Item" })]).meta({ title: "cli.command.swarms.list.ResponseItem" });

// src/viewer/command/swarms/list.ts
function swarmsListExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "swarms/list" }), zod.z.union([CliErrorSchema, CliCommandSwarmsListResponseItemSchema]));
}
function swarmsListExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "swarms/list" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function swarmsListRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "swarms/list/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsListRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "swarms/list/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsListResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "swarms/list/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms list response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsListResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "swarms/list/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms list response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandSwarmsPublishResponseSchema = zod.z.object({
  sha: zod.z.string()
}).meta({ title: "cli.command.swarms.publish.Response" });

// src/viewer/command/swarms/publish.ts
async function swarmsPublishExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "swarms/publish" }), zod.z.union([CliErrorSchema, CliCommandSwarmsPublishResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms publish: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsPublishExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "swarms/publish" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms publish: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsPublishRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "swarms/publish/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms publish request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsPublishRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "swarms/publish/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms publish request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsPublishResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "swarms/publish/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms publish response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsPublishResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "swarms/publish/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms publish response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandTasksListPluginSchema = zod.z.object({
  owner: zod.z.string(),
  repository: zod.z.string(),
  version: zod.z.string()
}).describe("The plugin that registered a schedule. All three fields are present\ntogether or the whole object is absent (the `schedules` table\nenforces all-or-nothing on its `plugin_*` columns).").meta({ title: "cli.command.tasks.list.Plugin" });

// src/cli/command/tasks/list/responseItem.ts
var CliCommandTasksListResponseItemSchema = zod.z.object({
  agent_instance_hierarchy: zod.z.string(),
  command: zod.z.array(zod.z.string()),
  created_at: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  description: zod.z.string(),
  id: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3).describe("The `schedules` row id. Monotonic; pass the highest `id` from a\npage as the next request's `after_id` to paginate forward."),
  interval: zod.z.string().nullable().describe('`None` for a oneshot; `Some("30s" / "1h" / "1d12h" / \u2026)`\nfor a recurring schedule, formatted as humantime so the\nlist output reads naturally without a unit-conversion\nstep at the consumer. The CLI parser accepts the same\nshape on `agents tasks schedule --interval`.').meta({ omitempty: true }).optional(),
  last_ran_at: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3).nullable().describe("Unix seconds of the most recent invocation \u2014 this row's newest\n`tasks_runs` entry. `None` until the runner has fired this\nversion at least once (runs are tracked per-version).").meta({ omitempty: true }).optional(),
  name: zod.z.string().describe("The `--name` passed to `agents tasks schedule`. Unique per\n`agent_instance_hierarchy`."),
  plugin: CliCommandTasksListPluginSchema.nullable().describe("The plugin that registered this schedule (its `(owner,\nrepository, version)` coordinate), or `None` when it was not\nscheduled by a plugin.").meta({ omitempty: true }).optional(),
  version: zod.z.number().int().min(0).max(18446744073709552e3).describe("This row's version: `1` for a freshly scheduled task,\n`max + 1` for each `tasks schedule --overwrite` (each version\nis its own row; only the newest lists).")
}).describe("One schedule row.").meta({ title: "cli.command.tasks.list.ResponseItem" });

// src/viewer/command/tasks/list.ts
function tasksListExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "tasks/list" }), zod.z.union([CliErrorSchema, CliCommandTasksListResponseItemSchema]));
}
function tasksListExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "tasks/list" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function tasksListRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "tasks/list/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tasks list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function tasksListRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "tasks/list/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tasks list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function tasksListResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "tasks/list/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tasks list response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function tasksListResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "tasks/list/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tasks list response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandTasksRunSuccessResponseItemSchema = zod.z.object({
  agent_instance_hierarchy: zod.z.string().describe("The source schedule's `agent_instance_hierarchy`."),
  name: zod.z.string().describe("The source schedule's `--name`."),
  plugin: CliCommandTasksListPluginSchema.nullable().describe("The plugin that registered the source schedule, if any.").meta({ omitempty: true }).optional(),
  success: zod.z.boolean().describe("Whether the task's stream completed without a trailing error."),
  version: zod.z.number().int().min(0).max(18446744073709552e3).describe("The source schedule's version (`1` on first creation,\nincremented per `schedule --overwrite`).")
}).describe("One per-task completion summary (default mode): the same schedule\nidentity as [`ValueResponseItem`], with `success` in lieu of\n`value`. `success` is `false` iff the task's FINAL emitted item was\nan error (a task that emitted nothing is a success). The task's\nfull output is in `tasks_logs` regardless.").meta({ title: "cli.command.tasks.run.SuccessResponseItem" });
var CliCommandTasksRunValueResponseItemSchema = zod.z.object({
  agent_instance_hierarchy: zod.z.string().describe("The source schedule's `agent_instance_hierarchy`."),
  name: zod.z.string().describe("The source schedule's `--name`."),
  plugin: CliCommandTasksListPluginSchema.nullable().describe("The plugin that registered the source schedule, if any.").meta({ omitempty: true }).optional(),
  value: JsonValueSchema.describe("The typed root item emitted by the scheduled command."),
  version: zod.z.number().int().min(0).max(18446744073709552e3).describe("The source schedule's version (`1` on first creation,\nincremented per `schedule --overwrite`).")
}).describe("One output item from one fired schedule's in-process stream\n(`stream_all` mode). The first four fields identify the source\nschedule; `value` is the typed root\n[`crate::cli::command::ResponseItem`] emitted by the scheduled cli\nleaf \u2014 boxed because the root union transitively contains *this*\ntype (`agents \u2192 tasks \u2192 run`), and boxing is what makes the\nrecursion sized.\n\nThe `value` field's JSON schema is opaqued to `serde_json::Value`\n(renders as bare `{}` aka JsonValue) so the published schema\ndoesn't inline the entire root union \u2014 that's the TS7056 blowup\nthe root and tier aggregates dodge by being `json_schema_ignore`.\nDownstream SDKs see `value: JsonValue` on the typed `execute`\npath; consumers that want to peer inside parse it case-by-case.").meta({ title: "cli.command.tasks.run.ValueResponseItem" });

// src/cli/command/tasks/run/responseItem.ts
var CliCommandTasksRunResponseItemSchema = zod.z.union([CliCommandTasksRunValueResponseItemSchema.meta({ "title": "cli.command.tasks.run.ValueResponseItem", "variantTitle": "Value" }), CliCommandTasksRunSuccessResponseItemSchema.meta({ "title": "cli.command.tasks.run.SuccessResponseItem", "variantTitle": "Success" })]).describe("One stream item from `tasks run`. Untagged \u2014 the variants'\nrequired fields (`value` vs `success`) are disjoint, so the wire\nshape is just the inner object. Which variant flows is decided by\nthe request's `stream_all`: `true` streams every emitted item as a\n[`ValueResponseItem`]; `false` (default) yields exactly one\n[`SuccessResponseItem`] per task when its stream completes.").meta({ title: "cli.command.tasks.run.ResponseItem" });

// src/viewer/command/tasks/run.ts
function tasksRunExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "tasks/run" }), zod.z.union([CliErrorSchema, CliCommandTasksRunResponseItemSchema]));
}
function tasksRunExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "tasks/run" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function tasksRunRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "tasks/run/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tasks run request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function tasksRunRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "tasks/run/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tasks run request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function tasksRunResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "tasks/run/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tasks run response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function tasksRunResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "tasks/run/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tasks run response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandTasksScheduleResponseSchema = zod.z.object({
  agent_instance_hierarchy: zod.z.string(),
  name: zod.z.string(),
  version: zod.z.number().int().min(0).max(18446744073709552e3)
}).describe("The created schedule's user-facing identity: its `--name`, the\ncaller hierarchy it was registered under, and the version this call\nminted (`1` on first creation, `max + 1` per `--overwrite` \u2014 each\nversion is its own row; older versions are shadowed but kept for\nper-version run history).").meta({ title: "cli.command.tasks.schedule.Response" });

// src/viewer/command/tasks/schedule.ts
async function tasksScheduleExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "tasks/schedule" }), zod.z.union([CliErrorSchema, CliCommandTasksScheduleResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tasks schedule: cli produced no output before the end marker");
  }
  return first;
}
async function tasksScheduleExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "tasks/schedule" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tasks schedule: cli produced no output before the end marker");
  }
  return first;
}
async function tasksScheduleRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "tasks/schedule/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tasks schedule request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function tasksScheduleRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "tasks/schedule/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tasks schedule request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function tasksScheduleResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "tasks/schedule/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tasks schedule response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function tasksScheduleResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "tasks/schedule/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tasks schedule response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandToolsGetExecSchema = zod.z.object({
  linux: zod.z.array(zod.z.string()),
  macos: zod.z.array(zod.z.string()),
  windows: zod.z.array(zod.z.string())
}).describe("Per-OS exec command for a tool. The current platform's vector is\nthe program plus its leading arguments; the caller's `--args` are\nappended, and the result runs with CWD = the tool's version folder\n(where `objectiveai.json` lives).").meta({ title: "cli.command.tools.get.Exec" });

// src/cli/command/tools/get/responseManifest.ts
var CliCommandToolsGetResponseManifestSchema = zod.z.object({
  description: zod.z.string(),
  exec: CliCommandToolsGetExecSchema,
  name: zod.z.string(),
  owner: zod.z.string(),
  source: zod.z.string(),
  version: zod.z.string()
}).meta({ title: "cli.command.tools.get.ResponseManifest" });

// src/viewer/command/tools/get.ts
async function toolsGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "tools/get" }), zod.z.union([CliErrorSchema, CliCommandToolsGetResponseManifestSchema.nullable()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools get: cli produced no output before the end marker");
  }
  return first;
}
async function toolsGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "tools/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools get: cli produced no output before the end marker");
  }
  return first;
}
async function toolsGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "tools/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function toolsGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "tools/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function toolsGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "tools/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function toolsGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "tools/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandToolsInstallResponseSchema = zod.z.object({
  instructions: zod.z.string()
}).meta({ title: "cli.command.tools.install.Response" });

// src/viewer/command/tools/install.ts
async function toolsInstallExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "tools/install" }), zod.z.union([CliErrorSchema, CliCommandToolsInstallResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools install: cli produced no output before the end marker");
  }
  return first;
}
async function toolsInstallExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "tools/install" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools install: cli produced no output before the end marker");
  }
  return first;
}
async function toolsInstallRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "tools/install/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools install request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function toolsInstallRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "tools/install/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools install request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function toolsInstallResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "tools/install/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools install response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function toolsInstallResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "tools/install/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools install response_schema: cli produced no output before the end marker");
  }
  return first;
}
function toolsListExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "tools/list" }), zod.z.union([CliErrorSchema, CliCommandToolsGetResponseManifestSchema]));
}
function toolsListExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "tools/list" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function toolsListRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "tools/list/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function toolsListRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "tools/list/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function toolsListResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "tools/list/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools list response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function toolsListResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "tools/list/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools list response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandToolsRunResponseItemSchema = zod.z.union([zod.z.string().meta({ "variantTitle": "Stdout" }), CliErrorSchema.meta({ "title": "cli.Error", "variantTitle": "Stderr" })]).meta({ title: "cli.command.tools.run.ResponseItem" });

// src/viewer/command/tools/run.ts
function toolsRunExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "tools/run" }), zod.z.union([CliErrorSchema, CliCommandToolsRunResponseItemSchema]));
}
function toolsRunExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "tools/run" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function toolsRunRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "tools/run/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools run request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function toolsRunRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "tools/run/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools run request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function toolsRunResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "tools/run/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools run response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function toolsRunResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "tools/run/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools run response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandUpdateResponseSkipReasonSchema = zod.z.enum(["dev_tree", "unsupported_platform", "incomplete_release"]).meta({ title: "cli.command.update.ResponseSkipReason" });

// src/cli/command/update/responseItem.ts
var CliCommandUpdateResponseItemSchema = zod.z.union([zod.z.object({
  asset_name: zod.z.string(),
  current_version: zod.z.string(),
  type: zod.z.literal("checking")
}).meta({ "variantTitle": "Checking" }), zod.z.object({
  asset_name: zod.z.string(),
  current_version: zod.z.string(),
  remote_version: zod.z.string(),
  type: zod.z.literal("found"),
  url: zod.z.string()
}).meta({ "variantTitle": "Found" }), zod.z.object({
  current_version: zod.z.string(),
  remote_version: zod.z.string(),
  type: zod.z.literal("installed")
}).meta({ "variantTitle": "Installed" }), zod.z.object({
  reason: CliCommandUpdateResponseSkipReasonSchema,
  type: zod.z.literal("skipped")
}).meta({ "variantTitle": "Skipped" }), zod.z.object({
  current_version: zod.z.string(),
  remote_version: zod.z.string(),
  type: zod.z.literal("up_to_date")
}).meta({ "variantTitle": "UpToDate" })]).describe("Per-stage update event. Update is a streaming leaf \u2014 one\n`ResponseItem` per (asset, stage) pair as the update progresses\nthrough the four shipped binaries (cli, api, viewer, mcp).").meta({ title: "cli.command.update.ResponseItem" });

// src/viewer/command/update.ts
function updateExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "update" }), zod.z.union([CliErrorSchema, CliCommandUpdateResponseItemSchema]));
}
function updateExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "update" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function updateRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "update/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("update request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function updateRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "update/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("update request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function updateResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "update/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("update response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function updateResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "update/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("update response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandViewerGenerateSecretSignaturePairResponseSchema = zod.z.object({
  secret: zod.z.string(),
  signature: zod.z.string()
}).meta({ title: "cli.command.viewer.generate_secret_signature_pair.Response" });

// src/viewer/command/viewer/generate_secret_signature_pair.ts
async function viewerGenerateSecretSignaturePairExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "viewer/generate_secret_signature_pair" }), zod.z.union([CliErrorSchema, CliCommandViewerGenerateSecretSignaturePairResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer generate_secret_signature_pair: cli produced no output before the end marker");
  }
  return first;
}
async function viewerGenerateSecretSignaturePairExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "viewer/generate_secret_signature_pair" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer generate_secret_signature_pair: cli produced no output before the end marker");
  }
  return first;
}
async function viewerGenerateSecretSignaturePairRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "viewer/generate_secret_signature_pair/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer generate_secret_signature_pair request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerGenerateSecretSignaturePairRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "viewer/generate_secret_signature_pair/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer generate_secret_signature_pair request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerGenerateSecretSignaturePairResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "viewer/generate_secret_signature_pair/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer generate_secret_signature_pair response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerGenerateSecretSignaturePairResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "viewer/generate_secret_signature_pair/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer generate_secret_signature_pair response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandViewerKillResponseSchema = zod.z.object({
  killed: zod.z.number().int().min(0).max(4294967295)
}).meta({ title: "cli.command.viewer.kill.Response" });

// src/viewer/command/viewer/kill.ts
async function viewerKillExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "viewer/kill" }), zod.z.union([CliErrorSchema, CliCommandViewerKillResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer kill: cli produced no output before the end marker");
  }
  return first;
}
async function viewerKillExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "viewer/kill" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer kill: cli produced no output before the end marker");
  }
  return first;
}
async function viewerKillRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "viewer/kill/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer kill request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerKillRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "viewer/kill/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer kill request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerKillResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "viewer/kill/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer kill response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerKillResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "viewer/kill/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer kill response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandViewerSendResponseSchema = zod.z.object({
  body: JsonValueSchema,
  status: zod.z.number().int().min(0).max(65535)
}).meta({ title: "cli.command.viewer.send.Response" });

// src/viewer/command/viewer/send.ts
async function viewerSendExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "viewer/send" }), zod.z.union([CliErrorSchema, CliCommandViewerSendResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer send: cli produced no output before the end marker");
  }
  return first;
}
async function viewerSendExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "viewer/send" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer send: cli produced no output before the end marker");
  }
  return first;
}
async function viewerSendRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "viewer/send/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer send request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerSendRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "viewer/send/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer send request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerSendResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "viewer/send/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer send response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerSendResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "viewer/send/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer send response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandViewerSpawnResponseSchema = zod.z.object({
  listening: zod.z.string()
}).meta({ title: "cli.command.viewer.spawn.Response" });

// src/viewer/command/viewer/spawn.ts
async function viewerSpawnExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "viewer/spawn" }), zod.z.union([CliErrorSchema, CliCommandViewerSpawnResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer spawn: cli produced no output before the end marker");
  }
  return first;
}
async function viewerSpawnExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "viewer/spawn" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer spawn: cli produced no output before the end marker");
  }
  return first;
}
async function viewerSpawnRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "viewer/spawn/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer spawn request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerSpawnRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "viewer/spawn/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer spawn request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerSpawnResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "viewer/spawn/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer spawn response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerSpawnResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "viewer/spawn/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer spawn response_schema: cli produced no output before the end marker");
  }
  return first;
}

// src/viewer/index.ts
function isInIframe() {
  return typeof window !== "undefined" && window.parent !== window;
}
var inboundListeners = /* @__PURE__ */ new Map();
var inboundHandlerAttached = false;
function attachInboundHandler() {
  if (inboundHandlerAttached) return;
  if (typeof window === "undefined") return;
  window.addEventListener("message", (event) => {
    const msg = event.data;
    if (!msg || typeof msg !== "object") return;
    if (msg.kind !== "plugin-event") return;
    if (msg.type !== "inbound") return;
    const set = inboundListeners.get(msg.sub_type);
    if (!set) return;
    for (const fn of set) {
      try {
        fn(msg.value);
      } catch (e) {
        console.error("@objectiveai/sdk/viewer listener threw:", e);
      }
    }
  });
  inboundHandlerAttached = true;
}
function listen2(sub_type, handler) {
  if (!isInIframe()) {
    let unlisten = null;
    let cancelled = false;
    void (async () => {
      try {
        const mod = await Promise.resolve().then(() => (init_event(), event_exports));
        if (cancelled) return;
        const u = await mod.listen(
          `plugin-${sub_type}`,
          (e) => handler(e.payload?.value)
        );
        if (cancelled) {
          u();
        } else {
          unlisten = u;
        }
      } catch {
        console.warn(
          `@objectiveai/sdk/viewer: listen('${sub_type}') called outside an iframe and @tauri-apps/api is unavailable; events will not fire.`
        );
      }
    })();
    return () => {
      cancelled = true;
      if (unlisten) unlisten();
    };
  }
  attachInboundHandler();
  const set = inboundListeners.get(sub_type) ?? /* @__PURE__ */ new Set();
  const fn = (value) => handler(value);
  set.add(fn);
  inboundListeners.set(sub_type, set);
  return () => {
    const s = inboundListeners.get(sub_type);
    if (!s) return;
    s.delete(fn);
    if (s.size === 0) inboundListeners.delete(sub_type);
  };
}
function __resetForTests() {
  inboundListeners.clear();
}

exports.CliStream = CliStream;
exports.ViewerEventSchema = ViewerEventSchema;
exports.__resetForTests = __resetForTests;
exports.agentsGetExecute = agentsGetExecute;
exports.agentsGetExecuteJq = agentsGetExecuteJq;
exports.agentsGetRequestSchemaExecute = agentsGetRequestSchemaExecute;
exports.agentsGetRequestSchemaExecuteJq = agentsGetRequestSchemaExecuteJq;
exports.agentsGetResponseSchemaExecute = agentsGetResponseSchemaExecute;
exports.agentsGetResponseSchemaExecuteJq = agentsGetResponseSchemaExecuteJq;
exports.agentsInstancesGetExecute = agentsInstancesGetExecute;
exports.agentsInstancesGetExecuteJq = agentsInstancesGetExecuteJq;
exports.agentsInstancesGetRequestSchemaExecute = agentsInstancesGetRequestSchemaExecute;
exports.agentsInstancesGetRequestSchemaExecuteJq = agentsInstancesGetRequestSchemaExecuteJq;
exports.agentsInstancesGetResponseSchemaExecute = agentsInstancesGetResponseSchemaExecute;
exports.agentsInstancesGetResponseSchemaExecuteJq = agentsInstancesGetResponseSchemaExecuteJq;
exports.agentsInstancesListExecute = agentsInstancesListExecute;
exports.agentsInstancesListExecuteJq = agentsInstancesListExecuteJq;
exports.agentsInstancesListRequestSchemaExecute = agentsInstancesListRequestSchemaExecute;
exports.agentsInstancesListRequestSchemaExecuteJq = agentsInstancesListRequestSchemaExecuteJq;
exports.agentsInstancesListResponseSchemaExecute = agentsInstancesListResponseSchemaExecute;
exports.agentsInstancesListResponseSchemaExecuteJq = agentsInstancesListResponseSchemaExecuteJq;
exports.agentsListExecute = agentsListExecute;
exports.agentsListExecuteJq = agentsListExecuteJq;
exports.agentsListRequestSchemaExecute = agentsListRequestSchemaExecute;
exports.agentsListRequestSchemaExecuteJq = agentsListRequestSchemaExecuteJq;
exports.agentsListResponseSchemaExecute = agentsListResponseSchemaExecute;
exports.agentsListResponseSchemaExecuteJq = agentsListResponseSchemaExecuteJq;
exports.agentsLogsReadAllExecute = agentsLogsReadAllExecute;
exports.agentsLogsReadAllExecuteJq = agentsLogsReadAllExecuteJq;
exports.agentsLogsReadAllRequestSchemaExecute = agentsLogsReadAllRequestSchemaExecute;
exports.agentsLogsReadAllRequestSchemaExecuteJq = agentsLogsReadAllRequestSchemaExecuteJq;
exports.agentsLogsReadAllResponseSchemaExecute = agentsLogsReadAllResponseSchemaExecute;
exports.agentsLogsReadAllResponseSchemaExecuteJq = agentsLogsReadAllResponseSchemaExecuteJq;
exports.agentsLogsReadIdExecute = agentsLogsReadIdExecute;
exports.agentsLogsReadIdExecuteJq = agentsLogsReadIdExecuteJq;
exports.agentsLogsReadIdRequestSchemaExecute = agentsLogsReadIdRequestSchemaExecute;
exports.agentsLogsReadIdRequestSchemaExecuteJq = agentsLogsReadIdRequestSchemaExecuteJq;
exports.agentsLogsReadIdResponseSchemaExecute = agentsLogsReadIdResponseSchemaExecute;
exports.agentsLogsReadIdResponseSchemaExecuteJq = agentsLogsReadIdResponseSchemaExecuteJq;
exports.agentsLogsReadPendingExecute = agentsLogsReadPendingExecute;
exports.agentsLogsReadPendingExecuteJq = agentsLogsReadPendingExecuteJq;
exports.agentsLogsReadPendingRequestSchemaExecute = agentsLogsReadPendingRequestSchemaExecute;
exports.agentsLogsReadPendingRequestSchemaExecuteJq = agentsLogsReadPendingRequestSchemaExecuteJq;
exports.agentsLogsReadPendingResponseSchemaExecute = agentsLogsReadPendingResponseSchemaExecute;
exports.agentsLogsReadPendingResponseSchemaExecuteJq = agentsLogsReadPendingResponseSchemaExecuteJq;
exports.agentsLogsReadSubscribeExecute = agentsLogsReadSubscribeExecute;
exports.agentsLogsReadSubscribeExecuteJq = agentsLogsReadSubscribeExecuteJq;
exports.agentsLogsReadSubscribeRequestSchemaExecute = agentsLogsReadSubscribeRequestSchemaExecute;
exports.agentsLogsReadSubscribeRequestSchemaExecuteJq = agentsLogsReadSubscribeRequestSchemaExecuteJq;
exports.agentsLogsReadSubscribeResponseSchemaExecute = agentsLogsReadSubscribeResponseSchemaExecute;
exports.agentsLogsReadSubscribeResponseSchemaExecuteJq = agentsLogsReadSubscribeResponseSchemaExecuteJq;
exports.agentsMessageExecute = agentsMessageExecute;
exports.agentsMessageExecuteJq = agentsMessageExecuteJq;
exports.agentsMessageExecuteStreaming = agentsMessageExecuteStreaming;
exports.agentsMessageExecuteStreamingJq = agentsMessageExecuteStreamingJq;
exports.agentsMessageRequestSchemaExecute = agentsMessageRequestSchemaExecute;
exports.agentsMessageRequestSchemaExecuteJq = agentsMessageRequestSchemaExecuteJq;
exports.agentsMessageResponseSchemaExecute = agentsMessageResponseSchemaExecute;
exports.agentsMessageResponseSchemaExecuteJq = agentsMessageResponseSchemaExecuteJq;
exports.agentsPublishExecute = agentsPublishExecute;
exports.agentsPublishExecuteJq = agentsPublishExecuteJq;
exports.agentsPublishRequestSchemaExecute = agentsPublishRequestSchemaExecute;
exports.agentsPublishRequestSchemaExecuteJq = agentsPublishRequestSchemaExecuteJq;
exports.agentsPublishResponseSchemaExecute = agentsPublishResponseSchemaExecute;
exports.agentsPublishResponseSchemaExecuteJq = agentsPublishResponseSchemaExecuteJq;
exports.agentsQueueDeleteExecute = agentsQueueDeleteExecute;
exports.agentsQueueDeleteExecuteJq = agentsQueueDeleteExecuteJq;
exports.agentsQueueDeleteRequestSchemaExecute = agentsQueueDeleteRequestSchemaExecute;
exports.agentsQueueDeleteRequestSchemaExecuteJq = agentsQueueDeleteRequestSchemaExecuteJq;
exports.agentsQueueDeleteResponseSchemaExecute = agentsQueueDeleteResponseSchemaExecute;
exports.agentsQueueDeleteResponseSchemaExecuteJq = agentsQueueDeleteResponseSchemaExecuteJq;
exports.agentsQueueDeliverExecute = agentsQueueDeliverExecute;
exports.agentsQueueDeliverExecuteJq = agentsQueueDeliverExecuteJq;
exports.agentsQueueDeliverRequestSchemaExecute = agentsQueueDeliverRequestSchemaExecute;
exports.agentsQueueDeliverRequestSchemaExecuteJq = agentsQueueDeliverRequestSchemaExecuteJq;
exports.agentsQueueDeliverResponseSchemaExecute = agentsQueueDeliverResponseSchemaExecute;
exports.agentsQueueDeliverResponseSchemaExecuteJq = agentsQueueDeliverResponseSchemaExecuteJq;
exports.agentsQueueReadIdExecute = agentsQueueReadIdExecute;
exports.agentsQueueReadIdExecuteJq = agentsQueueReadIdExecuteJq;
exports.agentsQueueReadIdRequestSchemaExecute = agentsQueueReadIdRequestSchemaExecute;
exports.agentsQueueReadIdRequestSchemaExecuteJq = agentsQueueReadIdRequestSchemaExecuteJq;
exports.agentsQueueReadIdResponseSchemaExecute = agentsQueueReadIdResponseSchemaExecute;
exports.agentsQueueReadIdResponseSchemaExecuteJq = agentsQueueReadIdResponseSchemaExecuteJq;
exports.agentsQueueReadPendingExecute = agentsQueueReadPendingExecute;
exports.agentsQueueReadPendingExecuteJq = agentsQueueReadPendingExecuteJq;
exports.agentsQueueReadPendingRequestSchemaExecute = agentsQueueReadPendingRequestSchemaExecute;
exports.agentsQueueReadPendingRequestSchemaExecuteJq = agentsQueueReadPendingRequestSchemaExecuteJq;
exports.agentsQueueReadPendingResponseSchemaExecute = agentsQueueReadPendingResponseSchemaExecute;
exports.agentsQueueReadPendingResponseSchemaExecuteJq = agentsQueueReadPendingResponseSchemaExecuteJq;
exports.agentsSpawnExecute = agentsSpawnExecute;
exports.agentsSpawnExecuteJq = agentsSpawnExecuteJq;
exports.agentsSpawnExecuteStreaming = agentsSpawnExecuteStreaming;
exports.agentsSpawnExecuteStreamingJq = agentsSpawnExecuteStreamingJq;
exports.agentsSpawnRequestSchemaExecute = agentsSpawnRequestSchemaExecute;
exports.agentsSpawnRequestSchemaExecuteJq = agentsSpawnRequestSchemaExecuteJq;
exports.agentsSpawnResponseSchemaExecute = agentsSpawnResponseSchemaExecute;
exports.agentsSpawnResponseSchemaExecuteJq = agentsSpawnResponseSchemaExecuteJq;
exports.agentsTagsApplyExecute = agentsTagsApplyExecute;
exports.agentsTagsApplyExecuteJq = agentsTagsApplyExecuteJq;
exports.agentsTagsApplyRequestSchemaExecute = agentsTagsApplyRequestSchemaExecute;
exports.agentsTagsApplyRequestSchemaExecuteJq = agentsTagsApplyRequestSchemaExecuteJq;
exports.agentsTagsApplyResponseSchemaExecute = agentsTagsApplyResponseSchemaExecute;
exports.agentsTagsApplyResponseSchemaExecuteJq = agentsTagsApplyResponseSchemaExecuteJq;
exports.agentsTagsLookupRequestSchemaExecute = agentsTagsLookupRequestSchemaExecute;
exports.agentsTagsLookupRequestSchemaExecuteJq = agentsTagsLookupRequestSchemaExecuteJq;
exports.agentsTagsLookupResponseSchemaExecute = agentsTagsLookupResponseSchemaExecute;
exports.agentsTagsLookupResponseSchemaExecuteJq = agentsTagsLookupResponseSchemaExecuteJq;
exports.configAgentsFavoritesAddExecute = configAgentsFavoritesAddExecute;
exports.configAgentsFavoritesAddExecuteJq = configAgentsFavoritesAddExecuteJq;
exports.configAgentsFavoritesAddRequestSchemaExecute = configAgentsFavoritesAddRequestSchemaExecute;
exports.configAgentsFavoritesAddRequestSchemaExecuteJq = configAgentsFavoritesAddRequestSchemaExecuteJq;
exports.configAgentsFavoritesAddResponseSchemaExecute = configAgentsFavoritesAddResponseSchemaExecute;
exports.configAgentsFavoritesAddResponseSchemaExecuteJq = configAgentsFavoritesAddResponseSchemaExecuteJq;
exports.configAgentsFavoritesDelExecute = configAgentsFavoritesDelExecute;
exports.configAgentsFavoritesDelExecuteJq = configAgentsFavoritesDelExecuteJq;
exports.configAgentsFavoritesDelRequestSchemaExecute = configAgentsFavoritesDelRequestSchemaExecute;
exports.configAgentsFavoritesDelRequestSchemaExecuteJq = configAgentsFavoritesDelRequestSchemaExecuteJq;
exports.configAgentsFavoritesDelResponseSchemaExecute = configAgentsFavoritesDelResponseSchemaExecute;
exports.configAgentsFavoritesDelResponseSchemaExecuteJq = configAgentsFavoritesDelResponseSchemaExecuteJq;
exports.configAgentsFavoritesEditExecute = configAgentsFavoritesEditExecute;
exports.configAgentsFavoritesEditExecuteJq = configAgentsFavoritesEditExecuteJq;
exports.configAgentsFavoritesEditRequestSchemaExecute = configAgentsFavoritesEditRequestSchemaExecute;
exports.configAgentsFavoritesEditRequestSchemaExecuteJq = configAgentsFavoritesEditRequestSchemaExecuteJq;
exports.configAgentsFavoritesEditResponseSchemaExecute = configAgentsFavoritesEditResponseSchemaExecute;
exports.configAgentsFavoritesEditResponseSchemaExecuteJq = configAgentsFavoritesEditResponseSchemaExecuteJq;
exports.configAgentsFavoritesGetExecute = configAgentsFavoritesGetExecute;
exports.configAgentsFavoritesGetExecuteJq = configAgentsFavoritesGetExecuteJq;
exports.configAgentsFavoritesGetRequestSchemaExecute = configAgentsFavoritesGetRequestSchemaExecute;
exports.configAgentsFavoritesGetRequestSchemaExecuteJq = configAgentsFavoritesGetRequestSchemaExecuteJq;
exports.configAgentsFavoritesGetResponseSchemaExecute = configAgentsFavoritesGetResponseSchemaExecute;
exports.configAgentsFavoritesGetResponseSchemaExecuteJq = configAgentsFavoritesGetResponseSchemaExecuteJq;
exports.configAgentsGetExecute = configAgentsGetExecute;
exports.configAgentsGetExecuteJq = configAgentsGetExecuteJq;
exports.configAgentsGetRequestSchemaExecute = configAgentsGetRequestSchemaExecute;
exports.configAgentsGetRequestSchemaExecuteJq = configAgentsGetRequestSchemaExecuteJq;
exports.configAgentsGetResponseSchemaExecute = configAgentsGetResponseSchemaExecute;
exports.configAgentsGetResponseSchemaExecuteJq = configAgentsGetResponseSchemaExecuteJq;
exports.configFunctionsFavoritesAddExecute = configFunctionsFavoritesAddExecute;
exports.configFunctionsFavoritesAddExecuteJq = configFunctionsFavoritesAddExecuteJq;
exports.configFunctionsFavoritesAddRequestSchemaExecute = configFunctionsFavoritesAddRequestSchemaExecute;
exports.configFunctionsFavoritesAddRequestSchemaExecuteJq = configFunctionsFavoritesAddRequestSchemaExecuteJq;
exports.configFunctionsFavoritesAddResponseSchemaExecute = configFunctionsFavoritesAddResponseSchemaExecute;
exports.configFunctionsFavoritesAddResponseSchemaExecuteJq = configFunctionsFavoritesAddResponseSchemaExecuteJq;
exports.configFunctionsFavoritesDelExecute = configFunctionsFavoritesDelExecute;
exports.configFunctionsFavoritesDelExecuteJq = configFunctionsFavoritesDelExecuteJq;
exports.configFunctionsFavoritesDelRequestSchemaExecute = configFunctionsFavoritesDelRequestSchemaExecute;
exports.configFunctionsFavoritesDelRequestSchemaExecuteJq = configFunctionsFavoritesDelRequestSchemaExecuteJq;
exports.configFunctionsFavoritesDelResponseSchemaExecute = configFunctionsFavoritesDelResponseSchemaExecute;
exports.configFunctionsFavoritesDelResponseSchemaExecuteJq = configFunctionsFavoritesDelResponseSchemaExecuteJq;
exports.configFunctionsFavoritesEditExecute = configFunctionsFavoritesEditExecute;
exports.configFunctionsFavoritesEditExecuteJq = configFunctionsFavoritesEditExecuteJq;
exports.configFunctionsFavoritesEditRequestSchemaExecute = configFunctionsFavoritesEditRequestSchemaExecute;
exports.configFunctionsFavoritesEditRequestSchemaExecuteJq = configFunctionsFavoritesEditRequestSchemaExecuteJq;
exports.configFunctionsFavoritesEditResponseSchemaExecute = configFunctionsFavoritesEditResponseSchemaExecute;
exports.configFunctionsFavoritesEditResponseSchemaExecuteJq = configFunctionsFavoritesEditResponseSchemaExecuteJq;
exports.configFunctionsFavoritesGetExecute = configFunctionsFavoritesGetExecute;
exports.configFunctionsFavoritesGetExecuteJq = configFunctionsFavoritesGetExecuteJq;
exports.configFunctionsFavoritesGetRequestSchemaExecute = configFunctionsFavoritesGetRequestSchemaExecute;
exports.configFunctionsFavoritesGetRequestSchemaExecuteJq = configFunctionsFavoritesGetRequestSchemaExecuteJq;
exports.configFunctionsFavoritesGetResponseSchemaExecute = configFunctionsFavoritesGetResponseSchemaExecute;
exports.configFunctionsFavoritesGetResponseSchemaExecuteJq = configFunctionsFavoritesGetResponseSchemaExecuteJq;
exports.configFunctionsGetExecute = configFunctionsGetExecute;
exports.configFunctionsGetExecuteJq = configFunctionsGetExecuteJq;
exports.configFunctionsGetRequestSchemaExecute = configFunctionsGetRequestSchemaExecute;
exports.configFunctionsGetRequestSchemaExecuteJq = configFunctionsGetRequestSchemaExecuteJq;
exports.configFunctionsGetResponseSchemaExecute = configFunctionsGetResponseSchemaExecute;
exports.configFunctionsGetResponseSchemaExecuteJq = configFunctionsGetResponseSchemaExecuteJq;
exports.configFunctionsProfilesFavoritesAddExecute = configFunctionsProfilesFavoritesAddExecute;
exports.configFunctionsProfilesFavoritesAddExecuteJq = configFunctionsProfilesFavoritesAddExecuteJq;
exports.configFunctionsProfilesFavoritesAddRequestSchemaExecute = configFunctionsProfilesFavoritesAddRequestSchemaExecute;
exports.configFunctionsProfilesFavoritesAddRequestSchemaExecuteJq = configFunctionsProfilesFavoritesAddRequestSchemaExecuteJq;
exports.configFunctionsProfilesFavoritesAddResponseSchemaExecute = configFunctionsProfilesFavoritesAddResponseSchemaExecute;
exports.configFunctionsProfilesFavoritesAddResponseSchemaExecuteJq = configFunctionsProfilesFavoritesAddResponseSchemaExecuteJq;
exports.configFunctionsProfilesFavoritesDelExecute = configFunctionsProfilesFavoritesDelExecute;
exports.configFunctionsProfilesFavoritesDelExecuteJq = configFunctionsProfilesFavoritesDelExecuteJq;
exports.configFunctionsProfilesFavoritesDelRequestSchemaExecute = configFunctionsProfilesFavoritesDelRequestSchemaExecute;
exports.configFunctionsProfilesFavoritesDelRequestSchemaExecuteJq = configFunctionsProfilesFavoritesDelRequestSchemaExecuteJq;
exports.configFunctionsProfilesFavoritesDelResponseSchemaExecute = configFunctionsProfilesFavoritesDelResponseSchemaExecute;
exports.configFunctionsProfilesFavoritesDelResponseSchemaExecuteJq = configFunctionsProfilesFavoritesDelResponseSchemaExecuteJq;
exports.configFunctionsProfilesFavoritesEditExecute = configFunctionsProfilesFavoritesEditExecute;
exports.configFunctionsProfilesFavoritesEditExecuteJq = configFunctionsProfilesFavoritesEditExecuteJq;
exports.configFunctionsProfilesFavoritesEditRequestSchemaExecute = configFunctionsProfilesFavoritesEditRequestSchemaExecute;
exports.configFunctionsProfilesFavoritesEditRequestSchemaExecuteJq = configFunctionsProfilesFavoritesEditRequestSchemaExecuteJq;
exports.configFunctionsProfilesFavoritesEditResponseSchemaExecute = configFunctionsProfilesFavoritesEditResponseSchemaExecute;
exports.configFunctionsProfilesFavoritesEditResponseSchemaExecuteJq = configFunctionsProfilesFavoritesEditResponseSchemaExecuteJq;
exports.configFunctionsProfilesFavoritesGetExecute = configFunctionsProfilesFavoritesGetExecute;
exports.configFunctionsProfilesFavoritesGetExecuteJq = configFunctionsProfilesFavoritesGetExecuteJq;
exports.configFunctionsProfilesFavoritesGetRequestSchemaExecute = configFunctionsProfilesFavoritesGetRequestSchemaExecute;
exports.configFunctionsProfilesFavoritesGetRequestSchemaExecuteJq = configFunctionsProfilesFavoritesGetRequestSchemaExecuteJq;
exports.configFunctionsProfilesFavoritesGetResponseSchemaExecute = configFunctionsProfilesFavoritesGetResponseSchemaExecute;
exports.configFunctionsProfilesFavoritesGetResponseSchemaExecuteJq = configFunctionsProfilesFavoritesGetResponseSchemaExecuteJq;
exports.configFunctionsProfilesGetExecute = configFunctionsProfilesGetExecute;
exports.configFunctionsProfilesGetExecuteJq = configFunctionsProfilesGetExecuteJq;
exports.configFunctionsProfilesGetRequestSchemaExecute = configFunctionsProfilesGetRequestSchemaExecute;
exports.configFunctionsProfilesGetRequestSchemaExecuteJq = configFunctionsProfilesGetRequestSchemaExecuteJq;
exports.configFunctionsProfilesGetResponseSchemaExecute = configFunctionsProfilesGetResponseSchemaExecute;
exports.configFunctionsProfilesGetResponseSchemaExecuteJq = configFunctionsProfilesGetResponseSchemaExecuteJq;
exports.configFunctionsProfilesPairsFavoritesAddExecute = configFunctionsProfilesPairsFavoritesAddExecute;
exports.configFunctionsProfilesPairsFavoritesAddExecuteJq = configFunctionsProfilesPairsFavoritesAddExecuteJq;
exports.configFunctionsProfilesPairsFavoritesAddRequestSchemaExecute = configFunctionsProfilesPairsFavoritesAddRequestSchemaExecute;
exports.configFunctionsProfilesPairsFavoritesAddRequestSchemaExecuteJq = configFunctionsProfilesPairsFavoritesAddRequestSchemaExecuteJq;
exports.configFunctionsProfilesPairsFavoritesAddResponseSchemaExecute = configFunctionsProfilesPairsFavoritesAddResponseSchemaExecute;
exports.configFunctionsProfilesPairsFavoritesAddResponseSchemaExecuteJq = configFunctionsProfilesPairsFavoritesAddResponseSchemaExecuteJq;
exports.configFunctionsProfilesPairsFavoritesDelExecute = configFunctionsProfilesPairsFavoritesDelExecute;
exports.configFunctionsProfilesPairsFavoritesDelExecuteJq = configFunctionsProfilesPairsFavoritesDelExecuteJq;
exports.configFunctionsProfilesPairsFavoritesDelRequestSchemaExecute = configFunctionsProfilesPairsFavoritesDelRequestSchemaExecute;
exports.configFunctionsProfilesPairsFavoritesDelRequestSchemaExecuteJq = configFunctionsProfilesPairsFavoritesDelRequestSchemaExecuteJq;
exports.configFunctionsProfilesPairsFavoritesDelResponseSchemaExecute = configFunctionsProfilesPairsFavoritesDelResponseSchemaExecute;
exports.configFunctionsProfilesPairsFavoritesDelResponseSchemaExecuteJq = configFunctionsProfilesPairsFavoritesDelResponseSchemaExecuteJq;
exports.configFunctionsProfilesPairsFavoritesEditExecute = configFunctionsProfilesPairsFavoritesEditExecute;
exports.configFunctionsProfilesPairsFavoritesEditExecuteJq = configFunctionsProfilesPairsFavoritesEditExecuteJq;
exports.configFunctionsProfilesPairsFavoritesEditRequestSchemaExecute = configFunctionsProfilesPairsFavoritesEditRequestSchemaExecute;
exports.configFunctionsProfilesPairsFavoritesEditRequestSchemaExecuteJq = configFunctionsProfilesPairsFavoritesEditRequestSchemaExecuteJq;
exports.configFunctionsProfilesPairsFavoritesEditResponseSchemaExecute = configFunctionsProfilesPairsFavoritesEditResponseSchemaExecute;
exports.configFunctionsProfilesPairsFavoritesEditResponseSchemaExecuteJq = configFunctionsProfilesPairsFavoritesEditResponseSchemaExecuteJq;
exports.configFunctionsProfilesPairsFavoritesGetExecute = configFunctionsProfilesPairsFavoritesGetExecute;
exports.configFunctionsProfilesPairsFavoritesGetExecuteJq = configFunctionsProfilesPairsFavoritesGetExecuteJq;
exports.configFunctionsProfilesPairsFavoritesGetRequestSchemaExecute = configFunctionsProfilesPairsFavoritesGetRequestSchemaExecute;
exports.configFunctionsProfilesPairsFavoritesGetRequestSchemaExecuteJq = configFunctionsProfilesPairsFavoritesGetRequestSchemaExecuteJq;
exports.configFunctionsProfilesPairsFavoritesGetResponseSchemaExecute = configFunctionsProfilesPairsFavoritesGetResponseSchemaExecute;
exports.configFunctionsProfilesPairsFavoritesGetResponseSchemaExecuteJq = configFunctionsProfilesPairsFavoritesGetResponseSchemaExecuteJq;
exports.configFunctionsProfilesPairsGetExecute = configFunctionsProfilesPairsGetExecute;
exports.configFunctionsProfilesPairsGetExecuteJq = configFunctionsProfilesPairsGetExecuteJq;
exports.configFunctionsProfilesPairsGetRequestSchemaExecute = configFunctionsProfilesPairsGetRequestSchemaExecute;
exports.configFunctionsProfilesPairsGetRequestSchemaExecuteJq = configFunctionsProfilesPairsGetRequestSchemaExecuteJq;
exports.configFunctionsProfilesPairsGetResponseSchemaExecute = configFunctionsProfilesPairsGetResponseSchemaExecute;
exports.configFunctionsProfilesPairsGetResponseSchemaExecuteJq = configFunctionsProfilesPairsGetResponseSchemaExecuteJq;
exports.configMcpAddressGetExecute = configMcpAddressGetExecute;
exports.configMcpAddressGetExecuteJq = configMcpAddressGetExecuteJq;
exports.configMcpAddressGetRequestSchemaExecute = configMcpAddressGetRequestSchemaExecute;
exports.configMcpAddressGetRequestSchemaExecuteJq = configMcpAddressGetRequestSchemaExecuteJq;
exports.configMcpAddressGetResponseSchemaExecute = configMcpAddressGetResponseSchemaExecute;
exports.configMcpAddressGetResponseSchemaExecuteJq = configMcpAddressGetResponseSchemaExecuteJq;
exports.configMcpAddressSetExecute = configMcpAddressSetExecute;
exports.configMcpAddressSetExecuteJq = configMcpAddressSetExecuteJq;
exports.configMcpAddressSetRequestSchemaExecute = configMcpAddressSetRequestSchemaExecute;
exports.configMcpAddressSetRequestSchemaExecuteJq = configMcpAddressSetRequestSchemaExecuteJq;
exports.configMcpAddressSetResponseSchemaExecute = configMcpAddressSetResponseSchemaExecute;
exports.configMcpAddressSetResponseSchemaExecuteJq = configMcpAddressSetResponseSchemaExecuteJq;
exports.configMcpGetExecute = configMcpGetExecute;
exports.configMcpGetExecuteJq = configMcpGetExecuteJq;
exports.configMcpGetRequestSchemaExecute = configMcpGetRequestSchemaExecute;
exports.configMcpGetRequestSchemaExecuteJq = configMcpGetRequestSchemaExecuteJq;
exports.configMcpGetResponseSchemaExecute = configMcpGetResponseSchemaExecute;
exports.configMcpGetResponseSchemaExecuteJq = configMcpGetResponseSchemaExecuteJq;
exports.configMcpPortGetExecute = configMcpPortGetExecute;
exports.configMcpPortGetExecuteJq = configMcpPortGetExecuteJq;
exports.configMcpPortGetRequestSchemaExecute = configMcpPortGetRequestSchemaExecute;
exports.configMcpPortGetRequestSchemaExecuteJq = configMcpPortGetRequestSchemaExecuteJq;
exports.configMcpPortGetResponseSchemaExecute = configMcpPortGetResponseSchemaExecute;
exports.configMcpPortGetResponseSchemaExecuteJq = configMcpPortGetResponseSchemaExecuteJq;
exports.configMcpPortSetExecute = configMcpPortSetExecute;
exports.configMcpPortSetExecuteJq = configMcpPortSetExecuteJq;
exports.configMcpPortSetRequestSchemaExecute = configMcpPortSetRequestSchemaExecute;
exports.configMcpPortSetRequestSchemaExecuteJq = configMcpPortSetRequestSchemaExecuteJq;
exports.configMcpPortSetResponseSchemaExecute = configMcpPortSetResponseSchemaExecute;
exports.configMcpPortSetResponseSchemaExecuteJq = configMcpPortSetResponseSchemaExecuteJq;
exports.configSwarmsFavoritesAddExecute = configSwarmsFavoritesAddExecute;
exports.configSwarmsFavoritesAddExecuteJq = configSwarmsFavoritesAddExecuteJq;
exports.configSwarmsFavoritesAddRequestSchemaExecute = configSwarmsFavoritesAddRequestSchemaExecute;
exports.configSwarmsFavoritesAddRequestSchemaExecuteJq = configSwarmsFavoritesAddRequestSchemaExecuteJq;
exports.configSwarmsFavoritesAddResponseSchemaExecute = configSwarmsFavoritesAddResponseSchemaExecute;
exports.configSwarmsFavoritesAddResponseSchemaExecuteJq = configSwarmsFavoritesAddResponseSchemaExecuteJq;
exports.configSwarmsFavoritesDelExecute = configSwarmsFavoritesDelExecute;
exports.configSwarmsFavoritesDelExecuteJq = configSwarmsFavoritesDelExecuteJq;
exports.configSwarmsFavoritesDelRequestSchemaExecute = configSwarmsFavoritesDelRequestSchemaExecute;
exports.configSwarmsFavoritesDelRequestSchemaExecuteJq = configSwarmsFavoritesDelRequestSchemaExecuteJq;
exports.configSwarmsFavoritesDelResponseSchemaExecute = configSwarmsFavoritesDelResponseSchemaExecute;
exports.configSwarmsFavoritesDelResponseSchemaExecuteJq = configSwarmsFavoritesDelResponseSchemaExecuteJq;
exports.configSwarmsFavoritesEditExecute = configSwarmsFavoritesEditExecute;
exports.configSwarmsFavoritesEditExecuteJq = configSwarmsFavoritesEditExecuteJq;
exports.configSwarmsFavoritesEditRequestSchemaExecute = configSwarmsFavoritesEditRequestSchemaExecute;
exports.configSwarmsFavoritesEditRequestSchemaExecuteJq = configSwarmsFavoritesEditRequestSchemaExecuteJq;
exports.configSwarmsFavoritesEditResponseSchemaExecute = configSwarmsFavoritesEditResponseSchemaExecute;
exports.configSwarmsFavoritesEditResponseSchemaExecuteJq = configSwarmsFavoritesEditResponseSchemaExecuteJq;
exports.configSwarmsFavoritesGetExecute = configSwarmsFavoritesGetExecute;
exports.configSwarmsFavoritesGetExecuteJq = configSwarmsFavoritesGetExecuteJq;
exports.configSwarmsFavoritesGetRequestSchemaExecute = configSwarmsFavoritesGetRequestSchemaExecute;
exports.configSwarmsFavoritesGetRequestSchemaExecuteJq = configSwarmsFavoritesGetRequestSchemaExecuteJq;
exports.configSwarmsFavoritesGetResponseSchemaExecute = configSwarmsFavoritesGetResponseSchemaExecute;
exports.configSwarmsFavoritesGetResponseSchemaExecuteJq = configSwarmsFavoritesGetResponseSchemaExecuteJq;
exports.configSwarmsGetExecute = configSwarmsGetExecute;
exports.configSwarmsGetExecuteJq = configSwarmsGetExecuteJq;
exports.configSwarmsGetRequestSchemaExecute = configSwarmsGetRequestSchemaExecute;
exports.configSwarmsGetRequestSchemaExecuteJq = configSwarmsGetRequestSchemaExecuteJq;
exports.configSwarmsGetResponseSchemaExecute = configSwarmsGetResponseSchemaExecute;
exports.configSwarmsGetResponseSchemaExecuteJq = configSwarmsGetResponseSchemaExecuteJq;
exports.configViewerAddressGetExecute = configViewerAddressGetExecute;
exports.configViewerAddressGetExecuteJq = configViewerAddressGetExecuteJq;
exports.configViewerAddressGetRequestSchemaExecute = configViewerAddressGetRequestSchemaExecute;
exports.configViewerAddressGetRequestSchemaExecuteJq = configViewerAddressGetRequestSchemaExecuteJq;
exports.configViewerAddressGetResponseSchemaExecute = configViewerAddressGetResponseSchemaExecute;
exports.configViewerAddressGetResponseSchemaExecuteJq = configViewerAddressGetResponseSchemaExecuteJq;
exports.configViewerAddressSetExecute = configViewerAddressSetExecute;
exports.configViewerAddressSetExecuteJq = configViewerAddressSetExecuteJq;
exports.configViewerAddressSetRequestSchemaExecute = configViewerAddressSetRequestSchemaExecute;
exports.configViewerAddressSetRequestSchemaExecuteJq = configViewerAddressSetRequestSchemaExecuteJq;
exports.configViewerAddressSetResponseSchemaExecute = configViewerAddressSetResponseSchemaExecute;
exports.configViewerAddressSetResponseSchemaExecuteJq = configViewerAddressSetResponseSchemaExecuteJq;
exports.configViewerGetExecute = configViewerGetExecute;
exports.configViewerGetExecuteJq = configViewerGetExecuteJq;
exports.configViewerGetRequestSchemaExecute = configViewerGetRequestSchemaExecute;
exports.configViewerGetRequestSchemaExecuteJq = configViewerGetRequestSchemaExecuteJq;
exports.configViewerGetResponseSchemaExecute = configViewerGetResponseSchemaExecute;
exports.configViewerGetResponseSchemaExecuteJq = configViewerGetResponseSchemaExecuteJq;
exports.configViewerPortGetExecute = configViewerPortGetExecute;
exports.configViewerPortGetExecuteJq = configViewerPortGetExecuteJq;
exports.configViewerPortGetRequestSchemaExecute = configViewerPortGetRequestSchemaExecute;
exports.configViewerPortGetRequestSchemaExecuteJq = configViewerPortGetRequestSchemaExecuteJq;
exports.configViewerPortGetResponseSchemaExecute = configViewerPortGetResponseSchemaExecute;
exports.configViewerPortGetResponseSchemaExecuteJq = configViewerPortGetResponseSchemaExecuteJq;
exports.configViewerPortSetExecute = configViewerPortSetExecute;
exports.configViewerPortSetExecuteJq = configViewerPortSetExecuteJq;
exports.configViewerPortSetRequestSchemaExecute = configViewerPortSetRequestSchemaExecute;
exports.configViewerPortSetRequestSchemaExecuteJq = configViewerPortSetRequestSchemaExecuteJq;
exports.configViewerPortSetResponseSchemaExecute = configViewerPortSetResponseSchemaExecute;
exports.configViewerPortSetResponseSchemaExecuteJq = configViewerPortSetResponseSchemaExecuteJq;
exports.configViewerSecretGetExecute = configViewerSecretGetExecute;
exports.configViewerSecretGetExecuteJq = configViewerSecretGetExecuteJq;
exports.configViewerSecretGetRequestSchemaExecute = configViewerSecretGetRequestSchemaExecute;
exports.configViewerSecretGetRequestSchemaExecuteJq = configViewerSecretGetRequestSchemaExecuteJq;
exports.configViewerSecretGetResponseSchemaExecute = configViewerSecretGetResponseSchemaExecute;
exports.configViewerSecretGetResponseSchemaExecuteJq = configViewerSecretGetResponseSchemaExecuteJq;
exports.configViewerSecretSetExecute = configViewerSecretSetExecute;
exports.configViewerSecretSetExecuteJq = configViewerSecretSetExecuteJq;
exports.configViewerSecretSetRequestSchemaExecute = configViewerSecretSetRequestSchemaExecute;
exports.configViewerSecretSetRequestSchemaExecuteJq = configViewerSecretSetRequestSchemaExecuteJq;
exports.configViewerSecretSetResponseSchemaExecute = configViewerSecretSetResponseSchemaExecute;
exports.configViewerSecretSetResponseSchemaExecuteJq = configViewerSecretSetResponseSchemaExecuteJq;
exports.configViewerSignatureGetExecute = configViewerSignatureGetExecute;
exports.configViewerSignatureGetExecuteJq = configViewerSignatureGetExecuteJq;
exports.configViewerSignatureGetRequestSchemaExecute = configViewerSignatureGetRequestSchemaExecute;
exports.configViewerSignatureGetRequestSchemaExecuteJq = configViewerSignatureGetRequestSchemaExecuteJq;
exports.configViewerSignatureGetResponseSchemaExecute = configViewerSignatureGetResponseSchemaExecute;
exports.configViewerSignatureGetResponseSchemaExecuteJq = configViewerSignatureGetResponseSchemaExecuteJq;
exports.configViewerSignatureSetExecute = configViewerSignatureSetExecute;
exports.configViewerSignatureSetExecuteJq = configViewerSignatureSetExecuteJq;
exports.configViewerSignatureSetRequestSchemaExecute = configViewerSignatureSetRequestSchemaExecute;
exports.configViewerSignatureSetRequestSchemaExecuteJq = configViewerSignatureSetRequestSchemaExecuteJq;
exports.configViewerSignatureSetResponseSchemaExecute = configViewerSignatureSetResponseSchemaExecute;
exports.configViewerSignatureSetResponseSchemaExecuteJq = configViewerSignatureSetResponseSchemaExecuteJq;
exports.dbQueryExecute = dbQueryExecute;
exports.dbQueryExecuteJq = dbQueryExecuteJq;
exports.dbQueryRequestSchemaExecute = dbQueryRequestSchemaExecute;
exports.dbQueryRequestSchemaExecuteJq = dbQueryRequestSchemaExecuteJq;
exports.dbQueryResponseSchemaExecute = dbQueryResponseSchemaExecute;
exports.dbQueryResponseSchemaExecuteJq = dbQueryResponseSchemaExecuteJq;
exports.functionsExecuteStandardExecute = functionsExecuteStandardExecute;
exports.functionsExecuteStandardExecuteJq = functionsExecuteStandardExecuteJq;
exports.functionsExecuteStandardExecuteStreaming = functionsExecuteStandardExecuteStreaming;
exports.functionsExecuteStandardExecuteStreamingJq = functionsExecuteStandardExecuteStreamingJq;
exports.functionsExecuteStandardRequestSchemaExecute = functionsExecuteStandardRequestSchemaExecute;
exports.functionsExecuteStandardRequestSchemaExecuteJq = functionsExecuteStandardRequestSchemaExecuteJq;
exports.functionsExecuteStandardResponseSchemaExecute = functionsExecuteStandardResponseSchemaExecute;
exports.functionsExecuteStandardResponseSchemaExecuteJq = functionsExecuteStandardResponseSchemaExecuteJq;
exports.functionsExecuteSwissSystemExecute = functionsExecuteSwissSystemExecute;
exports.functionsExecuteSwissSystemExecuteJq = functionsExecuteSwissSystemExecuteJq;
exports.functionsExecuteSwissSystemExecuteStreaming = functionsExecuteSwissSystemExecuteStreaming;
exports.functionsExecuteSwissSystemExecuteStreamingJq = functionsExecuteSwissSystemExecuteStreamingJq;
exports.functionsExecuteSwissSystemRequestSchemaExecute = functionsExecuteSwissSystemRequestSchemaExecute;
exports.functionsExecuteSwissSystemRequestSchemaExecuteJq = functionsExecuteSwissSystemRequestSchemaExecuteJq;
exports.functionsExecuteSwissSystemResponseSchemaExecute = functionsExecuteSwissSystemResponseSchemaExecute;
exports.functionsExecuteSwissSystemResponseSchemaExecuteJq = functionsExecuteSwissSystemResponseSchemaExecuteJq;
exports.functionsGetExecute = functionsGetExecute;
exports.functionsGetExecuteJq = functionsGetExecuteJq;
exports.functionsGetRequestSchemaExecute = functionsGetRequestSchemaExecute;
exports.functionsGetRequestSchemaExecuteJq = functionsGetRequestSchemaExecuteJq;
exports.functionsGetResponseSchemaExecute = functionsGetResponseSchemaExecute;
exports.functionsGetResponseSchemaExecuteJq = functionsGetResponseSchemaExecuteJq;
exports.functionsListExecute = functionsListExecute;
exports.functionsListExecuteJq = functionsListExecuteJq;
exports.functionsListRequestSchemaExecute = functionsListRequestSchemaExecute;
exports.functionsListRequestSchemaExecuteJq = functionsListRequestSchemaExecuteJq;
exports.functionsListResponseSchemaExecute = functionsListResponseSchemaExecute;
exports.functionsListResponseSchemaExecuteJq = functionsListResponseSchemaExecuteJq;
exports.functionsProfilesGetExecute = functionsProfilesGetExecute;
exports.functionsProfilesGetExecuteJq = functionsProfilesGetExecuteJq;
exports.functionsProfilesGetRequestSchemaExecute = functionsProfilesGetRequestSchemaExecute;
exports.functionsProfilesGetRequestSchemaExecuteJq = functionsProfilesGetRequestSchemaExecuteJq;
exports.functionsProfilesGetResponseSchemaExecute = functionsProfilesGetResponseSchemaExecute;
exports.functionsProfilesGetResponseSchemaExecuteJq = functionsProfilesGetResponseSchemaExecuteJq;
exports.functionsProfilesListExecute = functionsProfilesListExecute;
exports.functionsProfilesListExecuteJq = functionsProfilesListExecuteJq;
exports.functionsProfilesListRequestSchemaExecute = functionsProfilesListRequestSchemaExecute;
exports.functionsProfilesListRequestSchemaExecuteJq = functionsProfilesListRequestSchemaExecuteJq;
exports.functionsProfilesListResponseSchemaExecute = functionsProfilesListResponseSchemaExecute;
exports.functionsProfilesListResponseSchemaExecuteJq = functionsProfilesListResponseSchemaExecuteJq;
exports.functionsProfilesPublishExecute = functionsProfilesPublishExecute;
exports.functionsProfilesPublishExecuteJq = functionsProfilesPublishExecuteJq;
exports.functionsProfilesPublishRequestSchemaExecute = functionsProfilesPublishRequestSchemaExecute;
exports.functionsProfilesPublishRequestSchemaExecuteJq = functionsProfilesPublishRequestSchemaExecuteJq;
exports.functionsProfilesPublishResponseSchemaExecute = functionsProfilesPublishResponseSchemaExecute;
exports.functionsProfilesPublishResponseSchemaExecuteJq = functionsProfilesPublishResponseSchemaExecuteJq;
exports.functionsPublishExecute = functionsPublishExecute;
exports.functionsPublishExecuteJq = functionsPublishExecuteJq;
exports.functionsPublishRequestSchemaExecute = functionsPublishRequestSchemaExecute;
exports.functionsPublishRequestSchemaExecuteJq = functionsPublishRequestSchemaExecuteJq;
exports.functionsPublishResponseSchemaExecute = functionsPublishResponseSchemaExecute;
exports.functionsPublishResponseSchemaExecuteJq = functionsPublishResponseSchemaExecuteJq;
exports.invokeCliRequest = invokeCliRequest;
exports.listen = listen2;
exports.mcpKillExecute = mcpKillExecute;
exports.mcpKillExecuteJq = mcpKillExecuteJq;
exports.mcpKillRequestSchemaExecute = mcpKillRequestSchemaExecute;
exports.mcpKillRequestSchemaExecuteJq = mcpKillRequestSchemaExecuteJq;
exports.mcpKillResponseSchemaExecute = mcpKillResponseSchemaExecute;
exports.mcpKillResponseSchemaExecuteJq = mcpKillResponseSchemaExecuteJq;
exports.mcpSpawnExecute = mcpSpawnExecute;
exports.mcpSpawnExecuteJq = mcpSpawnExecuteJq;
exports.mcpSpawnRequestSchemaExecute = mcpSpawnRequestSchemaExecute;
exports.mcpSpawnRequestSchemaExecuteJq = mcpSpawnRequestSchemaExecuteJq;
exports.mcpSpawnResponseSchemaExecute = mcpSpawnResponseSchemaExecute;
exports.mcpSpawnResponseSchemaExecuteJq = mcpSpawnResponseSchemaExecuteJq;
exports.pluginsGetExecute = pluginsGetExecute;
exports.pluginsGetExecuteJq = pluginsGetExecuteJq;
exports.pluginsGetRequestSchemaExecute = pluginsGetRequestSchemaExecute;
exports.pluginsGetRequestSchemaExecuteJq = pluginsGetRequestSchemaExecuteJq;
exports.pluginsGetResponseSchemaExecute = pluginsGetResponseSchemaExecute;
exports.pluginsGetResponseSchemaExecuteJq = pluginsGetResponseSchemaExecuteJq;
exports.pluginsInstallFilesystemExecute = pluginsInstallFilesystemExecute;
exports.pluginsInstallFilesystemExecuteJq = pluginsInstallFilesystemExecuteJq;
exports.pluginsInstallFilesystemRequestSchemaExecute = pluginsInstallFilesystemRequestSchemaExecute;
exports.pluginsInstallFilesystemRequestSchemaExecuteJq = pluginsInstallFilesystemRequestSchemaExecuteJq;
exports.pluginsInstallFilesystemResponseSchemaExecute = pluginsInstallFilesystemResponseSchemaExecute;
exports.pluginsInstallFilesystemResponseSchemaExecuteJq = pluginsInstallFilesystemResponseSchemaExecuteJq;
exports.pluginsInstallGithubExecute = pluginsInstallGithubExecute;
exports.pluginsInstallGithubExecuteJq = pluginsInstallGithubExecuteJq;
exports.pluginsInstallGithubRequestSchemaExecute = pluginsInstallGithubRequestSchemaExecute;
exports.pluginsInstallGithubRequestSchemaExecuteJq = pluginsInstallGithubRequestSchemaExecuteJq;
exports.pluginsInstallGithubResponseSchemaExecute = pluginsInstallGithubResponseSchemaExecute;
exports.pluginsInstallGithubResponseSchemaExecuteJq = pluginsInstallGithubResponseSchemaExecuteJq;
exports.pluginsListExecute = pluginsListExecute;
exports.pluginsListExecuteJq = pluginsListExecuteJq;
exports.pluginsListRequestSchemaExecute = pluginsListRequestSchemaExecute;
exports.pluginsListRequestSchemaExecuteJq = pluginsListRequestSchemaExecuteJq;
exports.pluginsListResponseSchemaExecute = pluginsListResponseSchemaExecute;
exports.pluginsListResponseSchemaExecuteJq = pluginsListResponseSchemaExecuteJq;
exports.pluginsRunExecute = pluginsRunExecute;
exports.pluginsRunExecuteJq = pluginsRunExecuteJq;
exports.pluginsRunRequestSchemaExecute = pluginsRunRequestSchemaExecute;
exports.pluginsRunRequestSchemaExecuteJq = pluginsRunRequestSchemaExecuteJq;
exports.pluginsRunResponseSchemaExecute = pluginsRunResponseSchemaExecute;
exports.pluginsRunResponseSchemaExecuteJq = pluginsRunResponseSchemaExecuteJq;
exports.swarmsGetExecute = swarmsGetExecute;
exports.swarmsGetExecuteJq = swarmsGetExecuteJq;
exports.swarmsGetRequestSchemaExecute = swarmsGetRequestSchemaExecute;
exports.swarmsGetRequestSchemaExecuteJq = swarmsGetRequestSchemaExecuteJq;
exports.swarmsGetResponseSchemaExecute = swarmsGetResponseSchemaExecute;
exports.swarmsGetResponseSchemaExecuteJq = swarmsGetResponseSchemaExecuteJq;
exports.swarmsListExecute = swarmsListExecute;
exports.swarmsListExecuteJq = swarmsListExecuteJq;
exports.swarmsListRequestSchemaExecute = swarmsListRequestSchemaExecute;
exports.swarmsListRequestSchemaExecuteJq = swarmsListRequestSchemaExecuteJq;
exports.swarmsListResponseSchemaExecute = swarmsListResponseSchemaExecute;
exports.swarmsListResponseSchemaExecuteJq = swarmsListResponseSchemaExecuteJq;
exports.swarmsPublishExecute = swarmsPublishExecute;
exports.swarmsPublishExecuteJq = swarmsPublishExecuteJq;
exports.swarmsPublishRequestSchemaExecute = swarmsPublishRequestSchemaExecute;
exports.swarmsPublishRequestSchemaExecuteJq = swarmsPublishRequestSchemaExecuteJq;
exports.swarmsPublishResponseSchemaExecute = swarmsPublishResponseSchemaExecute;
exports.swarmsPublishResponseSchemaExecuteJq = swarmsPublishResponseSchemaExecuteJq;
exports.tasksListExecute = tasksListExecute;
exports.tasksListExecuteJq = tasksListExecuteJq;
exports.tasksListRequestSchemaExecute = tasksListRequestSchemaExecute;
exports.tasksListRequestSchemaExecuteJq = tasksListRequestSchemaExecuteJq;
exports.tasksListResponseSchemaExecute = tasksListResponseSchemaExecute;
exports.tasksListResponseSchemaExecuteJq = tasksListResponseSchemaExecuteJq;
exports.tasksRunExecute = tasksRunExecute;
exports.tasksRunExecuteJq = tasksRunExecuteJq;
exports.tasksRunRequestSchemaExecute = tasksRunRequestSchemaExecute;
exports.tasksRunRequestSchemaExecuteJq = tasksRunRequestSchemaExecuteJq;
exports.tasksRunResponseSchemaExecute = tasksRunResponseSchemaExecute;
exports.tasksRunResponseSchemaExecuteJq = tasksRunResponseSchemaExecuteJq;
exports.tasksScheduleExecute = tasksScheduleExecute;
exports.tasksScheduleExecuteJq = tasksScheduleExecuteJq;
exports.tasksScheduleRequestSchemaExecute = tasksScheduleRequestSchemaExecute;
exports.tasksScheduleRequestSchemaExecuteJq = tasksScheduleRequestSchemaExecuteJq;
exports.tasksScheduleResponseSchemaExecute = tasksScheduleResponseSchemaExecute;
exports.tasksScheduleResponseSchemaExecuteJq = tasksScheduleResponseSchemaExecuteJq;
exports.toolsGetExecute = toolsGetExecute;
exports.toolsGetExecuteJq = toolsGetExecuteJq;
exports.toolsGetRequestSchemaExecute = toolsGetRequestSchemaExecute;
exports.toolsGetRequestSchemaExecuteJq = toolsGetRequestSchemaExecuteJq;
exports.toolsGetResponseSchemaExecute = toolsGetResponseSchemaExecute;
exports.toolsGetResponseSchemaExecuteJq = toolsGetResponseSchemaExecuteJq;
exports.toolsInstallExecute = toolsInstallExecute;
exports.toolsInstallExecuteJq = toolsInstallExecuteJq;
exports.toolsInstallRequestSchemaExecute = toolsInstallRequestSchemaExecute;
exports.toolsInstallRequestSchemaExecuteJq = toolsInstallRequestSchemaExecuteJq;
exports.toolsInstallResponseSchemaExecute = toolsInstallResponseSchemaExecute;
exports.toolsInstallResponseSchemaExecuteJq = toolsInstallResponseSchemaExecuteJq;
exports.toolsListExecute = toolsListExecute;
exports.toolsListExecuteJq = toolsListExecuteJq;
exports.toolsListRequestSchemaExecute = toolsListRequestSchemaExecute;
exports.toolsListRequestSchemaExecuteJq = toolsListRequestSchemaExecuteJq;
exports.toolsListResponseSchemaExecute = toolsListResponseSchemaExecute;
exports.toolsListResponseSchemaExecuteJq = toolsListResponseSchemaExecuteJq;
exports.toolsRunExecute = toolsRunExecute;
exports.toolsRunExecuteJq = toolsRunExecuteJq;
exports.toolsRunRequestSchemaExecute = toolsRunRequestSchemaExecute;
exports.toolsRunRequestSchemaExecuteJq = toolsRunRequestSchemaExecuteJq;
exports.toolsRunResponseSchemaExecute = toolsRunResponseSchemaExecute;
exports.toolsRunResponseSchemaExecuteJq = toolsRunResponseSchemaExecuteJq;
exports.updateExecute = updateExecute;
exports.updateExecuteJq = updateExecuteJq;
exports.updateRequestSchemaExecute = updateRequestSchemaExecute;
exports.updateRequestSchemaExecuteJq = updateRequestSchemaExecuteJq;
exports.updateResponseSchemaExecute = updateResponseSchemaExecute;
exports.updateResponseSchemaExecuteJq = updateResponseSchemaExecuteJq;
exports.viewerGenerateSecretSignaturePairExecute = viewerGenerateSecretSignaturePairExecute;
exports.viewerGenerateSecretSignaturePairExecuteJq = viewerGenerateSecretSignaturePairExecuteJq;
exports.viewerGenerateSecretSignaturePairRequestSchemaExecute = viewerGenerateSecretSignaturePairRequestSchemaExecute;
exports.viewerGenerateSecretSignaturePairRequestSchemaExecuteJq = viewerGenerateSecretSignaturePairRequestSchemaExecuteJq;
exports.viewerGenerateSecretSignaturePairResponseSchemaExecute = viewerGenerateSecretSignaturePairResponseSchemaExecute;
exports.viewerGenerateSecretSignaturePairResponseSchemaExecuteJq = viewerGenerateSecretSignaturePairResponseSchemaExecuteJq;
exports.viewerKillExecute = viewerKillExecute;
exports.viewerKillExecuteJq = viewerKillExecuteJq;
exports.viewerKillRequestSchemaExecute = viewerKillRequestSchemaExecute;
exports.viewerKillRequestSchemaExecuteJq = viewerKillRequestSchemaExecuteJq;
exports.viewerKillResponseSchemaExecute = viewerKillResponseSchemaExecute;
exports.viewerKillResponseSchemaExecuteJq = viewerKillResponseSchemaExecuteJq;
exports.viewerSendExecute = viewerSendExecute;
exports.viewerSendExecuteJq = viewerSendExecuteJq;
exports.viewerSendRequestSchemaExecute = viewerSendRequestSchemaExecute;
exports.viewerSendRequestSchemaExecuteJq = viewerSendRequestSchemaExecuteJq;
exports.viewerSendResponseSchemaExecute = viewerSendResponseSchemaExecute;
exports.viewerSendResponseSchemaExecuteJq = viewerSendResponseSchemaExecuteJq;
exports.viewerSpawnExecute = viewerSpawnExecute;
exports.viewerSpawnExecuteJq = viewerSpawnExecuteJq;
exports.viewerSpawnRequestSchemaExecute = viewerSpawnRequestSchemaExecute;
exports.viewerSpawnRequestSchemaExecuteJq = viewerSpawnRequestSchemaExecuteJq;
exports.viewerSpawnResponseSchemaExecute = viewerSpawnResponseSchemaExecute;
exports.viewerSpawnResponseSchemaExecuteJq = viewerSpawnResponseSchemaExecuteJq;
