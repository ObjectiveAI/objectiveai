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
}).describe("Host \u2192 iframe. Carries data into the plugin (the existing\npath). `sub_type` is the snake_case discriminator the plugin\nlistens on (built-ins: `agent_completions` /\n`functions_executions`; plugins: whatever they declared in\ntheir manifest's `viewer_routes[i].type`).").meta({ "variantTitle": "Inbound" }), zod.z.object({
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
var CliCommandAgentsEnqueueResponseSchema = zod.z.object({
  agent_instance_hierarchy: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  agent_tag: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  id: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3)
}).describe("The freshly parked queue row: its id and the target it's scoped\nto (exactly one of `agent_instance_hierarchy` / `agent_tag` is\nset).").meta({ title: "cli.command.agents.enqueue.Response" });
var CliErrorTypeSchema = zod.z.literal("error").describe('Single-variant discriminator for [`Error`]\'s `type` field.\nAlways `"error"` on the wire.').meta({ title: "cli.ErrorType" });
var CliLevelSchema = zod.z.enum(["trace", "debug", "info", "warn", "error"]).describe("Severity matching the conventions used by bunyan / pino / `log` crate\nJSON encoders. `fatal` is encoded separately on [`Error`].").meta({ title: "cli.Level" });

// src/cli/error.ts
var CliErrorSchema = zod.z.object({
  fatal: zod.z.boolean().nullable().meta({ omitempty: true }).optional(),
  level: CliLevelSchema.nullable().meta({ omitempty: true }).optional(),
  message: JsonValueSchema,
  type: CliErrorTypeSchema
}).describe('A failure or advisory written to stdout. `fatal: true` means the\nprocess is exiting with a non-zero status; `fatal: false` is a\nnon-blocking warning (e.g. auto-update failed but the requested\ncommand still ran).\n\n`message` is an arbitrary JSON value so producers can emit\nstructured payloads (e.g. `{"code": ..., "detail": ...}`). Wrap\na plain string as `Value::String(...)` (or use `.into()`) and the\nwire bytes stay identical to the old `String`-only shape.\n\nThe `type` field is a single-variant `ErrorType` enum that\nalways serializes to `"error"`. This is what disambiguates the\nuntagged `Output` enum from a notification \u2014 `Output` tries\n`Error` first, and the constant `type:"error"` tag is what\nrejects every non-error wire shape.').meta({ title: "cli.Error" });

// src/viewer/command/agents/enqueue.ts
async function agentsEnqueueExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/enqueue" }), zod.z.union([CliErrorSchema, CliCommandAgentsEnqueueResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents enqueue: cli produced no output before the end marker");
  }
  return first;
}
async function agentsEnqueueExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/enqueue" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents enqueue: cli produced no output before the end marker");
  }
  return first;
}
async function agentsEnqueueRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/enqueue/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents enqueue request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsEnqueueRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/enqueue/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents enqueue request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsEnqueueResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/enqueue/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents enqueue response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsEnqueueResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/enqueue/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents enqueue response_schema: cli produced no output before the end marker");
  }
  return first;
}
var RemotePathSchema = zod.z.union([zod.z.object({
  commit: zod.z.string(),
  owner: zod.z.string(),
  remote: zod.z.literal("github"),
  repository: zod.z.string()
}).meta({ "variantTitle": "Github" }), zod.z.object({
  commit: zod.z.string(),
  owner: zod.z.string(),
  remote: zod.z.literal("client"),
  repository: zod.z.string()
}).meta({ "variantTitle": "Client" }), zod.z.object({
  name: zod.z.string(),
  remote: zod.z.literal("mock")
}).meta({ "variantTitle": "Mock" })]).meta({ title: "RemotePath" });

// src/cli/command/agents/get/response.ts
var CliCommandAgentsGetResponseSchema = RemotePathSchema.and(zod.z.object({})).describe("Response for `agents get` \u2014 an Agent base definition with its remote path.").meta({ title: "cli.command.agents.get.Response" });

// src/viewer/command/agents/get.ts
async function agentsGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/get" }), zod.z.union([CliErrorSchema, CliCommandAgentsGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents get: cli produced no output before the end marker");
  }
  return first;
}
async function agentsGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents get: cli produced no output before the end marker");
  }
  return first;
}
async function agentsGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandAgentsInstancesListResponseItemSchema = zod.z.object({
  agent_instance_hierarchy: zod.z.string().describe("Full hierarchy of this agent instance."),
  created_at: zod.z.string().nullable().describe("RFC3339 timestamp of the first `logs.messages` row for this\nagent. `None` when the agent has no logs yet (queue-only).").meta({ omitempty: true }).optional(),
  last_active_at: zod.z.string().nullable().describe("RFC3339 timestamp of the most recent `logs.messages` row for\nthis agent. `None` when the agent has no logs yet (queue-only).").meta({ omitempty: true }).optional(),
  logged: zod.z.number().int().min(0).max(18446744073709552e3).describe("Total `logs.messages` rows for this agent over all time."),
  queued: zod.z.number().int().min(0).max(18446744073709552e3).describe("Active `message_queue` rows targeting this agent \u2014 counting\nboth direct-AIH rows and rows whose tag is bound to this AIH."),
  tags: zod.z.array(zod.z.string()).describe("Tag names currently bound to this AIH, newest-bound first.")
}).describe("One discovered agent instance under a target. Aggregated from the\n`logs.messages`, `message_queue`, and `tags` tiers.").meta({ title: "cli.command.agents.instances.list.ResponseItem" });

// src/viewer/command/agents/instances/get.ts
function agentsInstancesGetExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/instances/get" }), zod.z.union([CliErrorSchema, CliCommandAgentsInstancesListResponseItemSchema]));
}
function agentsInstancesGetExecuteTransform(request, transform) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/instances/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function agentsInstancesGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/instances/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents instances get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsInstancesGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/instances/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents instances get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsInstancesGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/instances/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents instances get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsInstancesGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/instances/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents instances get response_schema: cli produced no output before the end marker");
  }
  return first;
}
function agentsInstancesListExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/instances/list" }), zod.z.union([CliErrorSchema, CliCommandAgentsInstancesListResponseItemSchema]));
}
function agentsInstancesListExecuteTransform(request, transform) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/instances/list" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function agentsInstancesListRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/instances/list/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents instances list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsInstancesListRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/instances/list/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents instances list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsInstancesListResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/instances/list/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents instances list response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsInstancesListResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/instances/list/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents instances list response_schema: cli produced no output before the end marker");
  }
  return first;
}
function agentsListExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/list" }), zod.z.union([CliErrorSchema, RemotePathSchema]));
}
function agentsListExecuteTransform(request, transform) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/list" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function agentsListRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/list/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsListRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/list/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsListResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/list/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents list response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsListResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/list/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents list response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandAgentsLogsReadAllAssistantResponsePartSchema = zod.z.union([zod.z.object({
  delivered_at: zod.z.string(),
  function_name: zod.z.string().describe("`objectiveai.assistant_response_tool_calls.function_name`."),
  id: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3).describe('`logs.messages."index"` for the tool-call row. Pass to\n`agents logs read id <n>` to read the call\'s `arguments`\nas text.'),
  tool_call_id: zod.z.string().describe("The wire tool-call id this row carries."),
  tool_call_index: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3).describe("The tool call's wire index within the assistant message's\n`tool_calls[]`."),
  type: zod.z.literal("tool_call")
}).meta({ "variantTitle": "ToolCall" }), zod.z.object({
  delivered_at: zod.z.string(),
  id: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  type: zod.z.literal("refusal")
}).meta({ "variantTitle": "Refusal" }), zod.z.object({
  delivered_at: zod.z.string(),
  id: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  type: zod.z.literal("reasoning")
}).meta({ "variantTitle": "Reasoning" }), zod.z.object({
  delivered_at: zod.z.string(),
  id: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  type: zod.z.literal("text")
}).meta({ "variantTitle": "Text" }), zod.z.object({
  delivered_at: zod.z.string(),
  id: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  type: zod.z.literal("image")
}).meta({ "variantTitle": "Image" }), zod.z.object({
  delivered_at: zod.z.string(),
  id: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  type: zod.z.literal("audio")
}).meta({ "variantTitle": "Audio" }), zod.z.object({
  delivered_at: zod.z.string(),
  id: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  type: zod.z.literal("video")
}).meta({ "variantTitle": "Video" }), zod.z.object({
  delivered_at: zod.z.string(),
  id: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  type: zod.z.literal("file")
}).meta({ "variantTitle": "File" })]).describe("One row inside an `AssistantResponse` block, tagged by the\ntable-kind of the underlying `assistant_response_*` row.\n\nThe `ToolCall` variant inlines the call's metadata\n(`function_name` / `tool_call_id` / `tool_call_index`) so callers\ncan dedupe and correlate without a per-row round-trip; its `id`\naddresses the same `assistant_response_tool_calls` row, which\n`agents logs read id <id>` returns as the call's `arguments`\n(text). Every other variant carries only `id` (the\n`logs.messages.\"index\"` to pass to `agents logs read id`) and the\ndelivery timestamp.").meta({ title: "cli.command.agents.logs.read.all.AssistantResponsePart" });
var CliCommandAgentsLogsReadAllClientNotificationPartTypeSchema = zod.z.enum(["text", "image", "audio", "video", "file"]).describe("Type tag for one `ClientNotification` part \u2014 the table-kind of\nthe underlying `message_queue_*` content row.").meta({ title: "cli.command.agents.logs.read.all.ClientNotificationPartType" });

// src/cli/command/agents/logs/read/all/clientNotificationPart.ts
var CliCommandAgentsLogsReadAllClientNotificationPartSchema = zod.z.object({
  delivered_at: zod.z.string().describe('`logs.messages."timestamp"` \u2014 when the receiver consumed\nthis content row and the LogWriter committed the\nconsumption event.'),
  id: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3).describe('`logs.messages."index"` for this row. Pass to\n`agents logs read id <n>` to fetch the consumed\n`message_queue_contents` body.'),
  type: CliCommandAgentsLogsReadAllClientNotificationPartTypeSchema
}).describe("One row inside a `ClientNotification` block \u2014 a consumed\n`message_queue_contents` entry. `queued_at` is on the\nenclosing block (it lives on `message_queue.enqueued_at`, not\nper-content); only the per-row consumption timestamp is here.").meta({ title: "cli.command.agents.logs.read.all.ClientNotificationPart" });
var CliCommandAgentsLogsReadAllToolResponsePartTypeSchema = zod.z.enum(["text", "image", "audio", "video", "file"]).describe("Type tag for one `ToolResponse` part \u2014 the table-kind of the\nunderlying `tool_response_content_*` row. The tool-call linkage\n(`tool_call_id`) lives on the enclosing `ResponseItem::ToolResponse`\nblock, not on the parts.").meta({ title: "cli.command.agents.logs.read.all.ToolResponsePartType" });

// src/cli/command/agents/logs/read/all/toolResponsePart.ts
var CliCommandAgentsLogsReadAllToolResponsePartSchema = zod.z.object({
  delivered_at: zod.z.string(),
  id: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3).describe('`logs.messages."index"` for this row. Pass to\n`agents logs read id <n>` for the typed body.'),
  type: CliCommandAgentsLogsReadAllToolResponsePartTypeSchema
}).describe("One row inside a `ToolResponse` block.").meta({ title: "cli.command.agents.logs.read.all.ToolResponsePart" });

// src/cli/command/agents/logs/read/all/responseItem.ts
var CliCommandAgentsLogsReadAllResponseItemSchema = zod.z.union([zod.z.object({
  agent_instance_hierarchy: zod.z.string(),
  delivered_at: zod.z.string(),
  id: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  response_id: zod.z.string(),
  sender_agent_instance_hierarchy: zod.z.string().describe("AIH of the caller who issued the request \u2014 from\n`logs.agent_completion_requests.sender_*`."),
  type: zod.z.literal("agent_completion_request")
}).meta({ "variantTitle": "AgentCompletionRequest" }), zod.z.object({
  agent_instance_hierarchy: zod.z.string(),
  delivered_at: zod.z.string(),
  id: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  response_id: zod.z.string(),
  sender_agent_instance_hierarchy: zod.z.string(),
  type: zod.z.literal("vector_completion_request")
}).meta({ "variantTitle": "VectorCompletionRequest" }), zod.z.object({
  agent_instance_hierarchy: zod.z.string(),
  delivered_at: zod.z.string(),
  id: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  response_id: zod.z.string(),
  sender_agent_instance_hierarchy: zod.z.string(),
  type: zod.z.literal("function_execution_request")
}).meta({ "variantTitle": "FunctionExecutionRequest" }), zod.z.object({
  agent_instance_hierarchy: zod.z.string(),
  key: zod.z.string().nullable().describe("Idempotency token, if the row was enqueued with\n`--key` via `agents message --enqueue-with-key`.\nSurfacing it lets readers attribute a notification\nto a specific enqueue beyond just the sender AIH.").meta({ omitempty: true }).optional(),
  parts: zod.z.array(CliCommandAgentsLogsReadAllClientNotificationPartSchema),
  queued_at: zod.z.string().describe("`message_queue.enqueued_at` of the consumed parent\nqueue row. One block = one parent queue row, so this\nis well-defined block-level (each part's individual\n`delivered_at` still records its own\nconsumption moment)."),
  response_id: zod.z.string(),
  sender_agent_instance_hierarchy: zod.z.string().describe("AIH of the enqueuer \u2014 from `message_queue.sender_*`\njoined through `message_queue_contents.id`."),
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
  tool_call_id: zod.z.string().describe("The wire tool-call id this response answers."),
  type: zod.z.literal("tool_response")
}).describe("One tool call's response. Blocks are grouped per\n`tool_call_id` (in addition to `agent_instance_hierarchy` +\n`response_id`), so two responses in the same turn yield two\nblocks.").meta({ "variantTitle": "ToolResponse" })]).describe("One yielded item. Three single-row request blobs +\nthree multi-row blocks. Every variant carries `response_id`.\n`sender_agent_instance_hierarchy` appears only on the four\nvariants that have a sender \u2260 producer: the three request\nvariants (caller AIH) and `ClientNotification` (enqueuer\nAIH). `AssistantResponse` and `ToolResponse` are emitted BY\nthe agent itself \u2014 their `agent_instance_hierarchy` IS the\nproducer, so no separate sender field exists.\n\nBlock-coalescing boundary tuple: `(class,\nagent_instance_hierarchy, response_id)` for `AssistantResponse`;\n`(class, agent_instance_hierarchy, response_id, tool_call_id)`\nfor `ToolResponse` (one block per tool call); `(class,\nagent_instance_hierarchy, response_id, sender, message_queue_id)`\nfor `ClientNotification` blocks.\nOne `ClientNotification` block = one consumed\n`message_queue` parent row, so `queued_at` and\n`sender_agent_instance_hierarchy` are well-defined\nblock-level.").meta({ title: "cli.command.agents.logs.read.all.ResponseItem" });

// src/viewer/command/agents/logs/read/all.ts
function agentsLogsReadAllExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/logs/read/all" }), zod.z.union([CliErrorSchema, CliCommandAgentsLogsReadAllResponseItemSchema]));
}
function agentsLogsReadAllExecuteTransform(request, transform) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/logs/read/all" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function agentsLogsReadAllRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/logs/read/all/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read all request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsLogsReadAllRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/logs/read/all/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read all request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsLogsReadAllResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/logs/read/all/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read all response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsLogsReadAllResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/logs/read/all/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
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
var AgentMockOutputModeSchema = zod.z.union([zod.z.literal("instruction").describe("The model is instructed via the prompt to output a specific key.\n\nThis is the default and most widely supported mode.").meta({ "variantTitle": "Instruction" }), zod.z.literal("json_schema").describe("A JSON schema response format is used with an enum of possible keys.\n\nRequires model support for structured JSON output.").meta({ "variantTitle": "JsonSchema" }), zod.z.literal("tool_call").describe("A forced tool call with an argument schema containing possible keys.\n\nRequires model support for tool/function calling.").meta({ "variantTitle": "ToolCall" })]).describe("The method used to constrain LLM output to valid response keys.\n\nIn vector completions, the model must select from a predefined set of\nresponses. This enum controls *how* that constraint is enforced.\n\n**Note:** This setting is only relevant for vector completions and is\ncompletely ignored for agent completions.").meta({ title: "agent.mock.OutputMode" });
var AgentMockUpstreamSchema = zod.z.literal("mock").describe("Mock upstream marker.").meta({ title: "agent.mock.Upstream" });

// src/agent/mock/agentBase.ts
var AgentMockAgentBaseSchema = zod.z.object({
  calls: zod.z.array(AgentMockCallSchema).nullable().describe("Deterministic-script override. When `Some`, the mock agent\nemits each [`super::Call`] as its own assistant turn \u2014\n`tool_calls` first, then `content` \u2014 in array order. Each\nsubsequent turn inspects the continuation to count how many\n`Call`s have already been satisfied (assistant message with\nexactly that `Call`'s `tool_calls` (by name+arguments) and\n`content`); the next un-matched `Call` is what that turn\nemits. Once every `Call` has been satisfied in the\ncontinuation, the mock falls through to its normal\ndispatcher. Pure addition \u2014 agents without `calls` are\nunaffected.").meta({ omitempty: true }).optional(),
  client_objectiveai_mcp: AgentClientObjectiveaiMcpSchema.nullable().describe("Client-side ObjectiveAI MCP surface the calling client is\nexpected to expose locally back to the API (objectiveai\nbuilt-in, plus specific plugins / tools by owner+name+version).").meta({ omitempty: true }).optional(),
  error: zod.z.boolean().nullable().describe("If true, the mock client will return an error instead of a response.").meta({ omitempty: true }).optional(),
  error_probability: zod.z.number().int().min(0).max(255).nullable().describe("Probability (0-100) that the mock returns an error mid-stream.\nRequires `error` to be `Some(true)`.").meta({ omitempty: true }).optional(),
  mcp_servers: zod.z.array(AgentMcpServerSchema).nullable().describe("MCP servers the agent can connect to.").meta({ omitempty: true }).optional(),
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
  remote: zod.z.literal("client"),
  repository: zod.z.string()
}).meta({ "variantTitle": "Client" }), zod.z.object({
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
  input: FunctionsExpressionExpressionSchema,
  input_schema: FunctionsExpressionObjectInputSchemaSchema,
  skip: FunctionsExpressionExpressionSchema.nullable().meta({ omitempty: true }).optional()
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
  input: FunctionsExpressionExpressionSchema,
  input_schema: FunctionsExpressionObjectInputSchemaSchema,
  skip: FunctionsExpressionExpressionSchema.nullable().meta({ omitempty: true }).optional()
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
  input: FunctionsAlphaVectorExpressionVectorFunctionInputValueExpressionSchema,
  input_schema: FunctionsAlphaVectorExpressionVectorFunctionInputSchemaSchema,
  skip: FunctionsExpressionExpressionSchema.nullable().meta({ omitempty: true }).optional()
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
  function: FunctionsFullInlineFunctionOrRemoteCommitOptionalSchema.describe("The function to execute (inline definition or remote path)."),
  input: FunctionsExpressionInputValueSchema,
  invert: zod.z.boolean().nullable().describe('If `true`, invert every output in the streamed response *after* the\ninner function has finished computing \u2014 scalar outputs become\n`1 - x`, vector outputs are reversed in place. The expression\nevaluator inside the function still sees the original scores; only\nthe chunks delivered to the client (and the aggregated response\npassed to the usage handler) are inverted. Useful when a function\nis naturally written to score "lower is better" but the consumer\nwants "higher is better", or vice versa.').meta({ omitempty: true }).optional(),
  profile: FunctionsInlineProfileOrRemoteCommitOptionalSchema.describe("The profile to use (inline definition or remote path)."),
  provider: AgentCompletionsRequestProviderSchema.nullable().meta({ omitempty: true }).optional(),
  reasoning: FunctionsExecutionsRequestReasoningSchema.nullable().meta({ omitempty: true }).optional(),
  seed: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3).nullable().meta({ omitempty: true }).optional(),
  split: zod.z.boolean().nullable().meta({ omitempty: true }).optional(),
  strategy: FunctionsExecutionsRequestStrategySchema.nullable().meta({ omitempty: true }).optional(),
  stream: zod.z.boolean().nullable().meta({ omitempty: true }).optional()
}).describe("Parameters for creating a function execution.").meta({ title: "functions.executions.request.FunctionExecutionCreateParams" });
var SwarmInlineSwarmBaseOrRemoteCommitOptionalSchema = zod.z.union([SwarmInlineSwarmBaseSchema.meta({ "title": "swarm.InlineSwarmBase", "variantTitle": "SwarmBase" }), RemotePathCommitOptionalSchema.meta({ "title": "RemotePathCommitOptional", "variantTitle": "Remote" })]).describe("Like [`InlineSwarmBaseOrRemote`] but with optional commit.\nUsed in request types where commit resolution happens server-side.").meta({ title: "swarm.InlineSwarmBaseOrRemoteCommitOptional" });

// src/vector/completions/request/vectorCompletionCreateParams.ts
var VectorCompletionsRequestVectorCompletionCreateParamsSchema = zod.z.object({
  continuation: zod.z.string().nullable().describe("Continuation from a previous completion, as a base64-encoded string.").meta({ omitempty: true }).optional(),
  messages: zod.z.array(AgentCompletionsMessageMessageSchema).describe("The conversation messages (the prompt)."),
  provider: AgentCompletionsRequestProviderSchema.nullable().describe("Provider routing preferences.").meta({ omitempty: true }).optional(),
  responses: zod.z.array(AgentCompletionsMessageRichContentSchema).describe("The possible responses the LLMs can vote for."),
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
  text: zod.z.string(),
  type: zod.z.literal("text")
}).meta({ "variantTitle": "Text" }), AgentCompletionsMessageImageUrlSchema.and(zod.z.object({
  type: zod.z.literal("image")
})).meta({ "variantTitle": "Image" }), AgentCompletionsMessageInputAudioSchema.and(zod.z.object({
  type: zod.z.literal("audio")
})).meta({ "variantTitle": "Audio" }), AgentCompletionsMessageVideoUrlSchema.and(zod.z.object({
  type: zod.z.literal("video")
})).meta({ "variantTitle": "Video" }), AgentCompletionsMessageFileSchema.and(zod.z.object({
  type: zod.z.literal("file")
})).meta({ "variantTitle": "File" })]).describe('Resolved payload for one `logs.messages."index"`. Tagged by\n`type`, snake_case discriminant. The MCP projection in\n[`CommandResponse::into_mcp`] hands media variants over as\n[`ContentBlock`]s and text as a bare JSON string \u2014 matching the\nexisting `agents queue read id` projection of `RichContentPart`.\nThe three request-blob variants render as JSONL with their full\ntyped `\u2026CreateParams` body so callers can introspect request\nmetadata.\n\n[`ContentBlock`]: crate::mcp::tool::ContentBlock').meta({ title: "cli.command.agents.logs.read.id.Response" });

// src/viewer/command/agents/logs/read/id.ts
async function agentsLogsReadIdExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/logs/read/id" }), zod.z.union([CliErrorSchema, CliCommandAgentsLogsReadIdResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read id: cli produced no output before the end marker");
  }
  return first;
}
async function agentsLogsReadIdExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/logs/read/id" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read id: cli produced no output before the end marker");
  }
  return first;
}
async function agentsLogsReadIdRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/logs/read/id/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read id request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsLogsReadIdRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/logs/read/id/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read id request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsLogsReadIdResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/logs/read/id/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read id response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsLogsReadIdResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/logs/read/id/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read id response_schema: cli produced no output before the end marker");
  }
  return first;
}
function agentsLogsReadPendingExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/logs/read/pending" }), zod.z.union([CliErrorSchema, CliCommandAgentsLogsReadAllResponseItemSchema]));
}
function agentsLogsReadPendingExecuteTransform(request, transform) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/logs/read/pending" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function agentsLogsReadPendingRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/logs/read/pending/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read pending request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsLogsReadPendingRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/logs/read/pending/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read pending request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsLogsReadPendingResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/logs/read/pending/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read pending response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsLogsReadPendingResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/logs/read/pending/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
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
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/logs/read/subscribe" }), zod.z.union([CliErrorSchema, CliCommandAgentsLogsReadSubscribeResponseItemSchema]));
}
function agentsLogsReadSubscribeExecuteTransform(request, transform) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/logs/read/subscribe" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function agentsLogsReadSubscribeRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/logs/read/subscribe/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsLogsReadSubscribeRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/logs/read/subscribe/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsLogsReadSubscribeResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/logs/read/subscribe/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsLogsReadSubscribeResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/logs/read/subscribe/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandAgentsMessageResponseSchema = zod.z.union([zod.z.object({
  type: zod.z.literal("delivered")
}).describe("The queue row reached a live agent (its row flipped to\ninactive \u2014 the API stamped its id onto an assistant chunk's\n`request_message_ids`) before the agent's lock freed up.").meta({ "variantTitle": "Delivered" }), zod.z.object({
  agent_instance_hierarchy: zod.z.string(),
  type: zod.z.literal("id")
}).describe("The handler execed a detached `agents spawn` child (with the\nagent's lock transferred into it) and the child yielded its\n`Id` first item \u2014 the bare `agent_instance_hierarchy` the\nrunner just minted or resumed.").meta({ "variantTitle": "Id" })]).describe('Unary response. Exactly one of these per call. Internally tagged\nvia `type`; bare unit variant `Delivered` serializes as\n`{"type":"delivered"}`.').meta({ title: "cli.command.agents.message.Response" });

// src/viewer/command/agents/message.ts
async function agentsMessageExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/message" }), zod.z.union([CliErrorSchema, CliCommandAgentsMessageResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents message: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMessageExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/message" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents message: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMessageRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/message/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents message request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMessageRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/message/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents message request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMessageResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/message/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents message response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMessageResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/message/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
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
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/publish" }), zod.z.union([CliErrorSchema, CliCommandAgentsPublishResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents publish: cli produced no output before the end marker");
  }
  return first;
}
async function agentsPublishExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/publish" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents publish: cli produced no output before the end marker");
  }
  return first;
}
async function agentsPublishRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/publish/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents publish request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsPublishRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/publish/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents publish request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsPublishResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/publish/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents publish response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsPublishResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/publish/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
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
  enqueued_at: zod.z.string().describe("RFC3339 timestamp the dropped row was enqueued at."),
  id: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  key: zod.z.string().nullable().describe("Idempotency token, if the dropped row had one.").meta({ omitempty: true }).optional()
}).describe("What was deleted. Carries every column of the original\n`prompts` row so the caller can confirm the drop:\nexactly one of `agent_instance_hierarchy` / `agent_tag` is set\n(matching the original target), `enqueued_at` is the original\nunix-seconds timestamp, and `content` is the reconstructed\n`RichContent` body.").meta({ title: "cli.command.agents.queue.delete.Response" });

// src/viewer/command/agents/queue/delete.ts
async function agentsQueueDeleteExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/queue/delete" }), zod.z.union([CliErrorSchema, CliCommandAgentsQueueDeleteResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue delete: cli produced no output before the end marker");
  }
  return first;
}
async function agentsQueueDeleteExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/queue/delete" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue delete: cli produced no output before the end marker");
  }
  return first;
}
async function agentsQueueDeleteRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/queue/delete/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue delete request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsQueueDeleteRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/queue/delete/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue delete request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsQueueDeleteResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/queue/delete/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue delete response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsQueueDeleteResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/queue/delete/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
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
var CliCommandAgentsQueueDeliverTagActiveTypeSchema = zod.z.literal("TagActive").meta({ title: "cli.command.agents.queue.deliver.TagActiveType" });

// src/cli/command/agents/queue/deliver/tagActiveResponseItem.ts
var CliCommandAgentsQueueDeliverTagActiveResponseItemSchema = zod.z.object({
  agent_tag: zod.z.string(),
  type: CliCommandAgentsQueueDeliverTagActiveTypeSchema
}).describe("This un-upgraded tag's lock was held by a live owner \u2014 another\nprocess is already materializing it; the queued rows will reach\nthe agent it mints. Nothing was spawned for it.").meta({ title: "cli.command.agents.queue.deliver.TagActiveResponseItem" });
var CliCommandAgentsQueueDeliverTagSpawnedTypeSchema = zod.z.literal("TagSpawned").meta({ title: "cli.command.agents.queue.deliver.TagSpawnedType" });

// src/cli/command/agents/queue/deliver/tagSpawnedResponseItem.ts
var CliCommandAgentsQueueDeliverTagSpawnedResponseItemSchema = zod.z.object({
  agent_tag: zod.z.string(),
  type: CliCommandAgentsQueueDeliverTagSpawnedTypeSchema
}).describe("This un-upgraded tag's lock was won and a fresh spawn of the\ngroup's stored agent spec has started. The minted\n`agent_instance_hierarchy` isn't known yet at this point \u2014 it\narrives as the FIRST inner item (the spawn `Id`) of the\n[`ValueResponseItem`]s that follow (in `stream_spawns` mode).").meta({ title: "cli.command.agents.queue.deliver.TagSpawnedResponseItem" });
var CliCommandAgentsQueueDeliverValueResponseItemSchema = zod.z.object({
  agent_instance_hierarchy: zod.z.string().describe("The delivered agent's `agent_instance_hierarchy`."),
  value: JsonValueSchema.describe("The typed root item the spawn emitted.")
}).describe("One output item from one delivered agent's spawn stream. `value`\nis the typed root [`crate::cli::command::ResponseItem`] (the spawn\nitem wrapped at the root) \u2014 boxed because the root union\ntransitively contains *this* type (`agents \u2192 queue \u2192 deliver`),\nand boxing is what makes the recursion sized.\n\nThe `value` field's JSON schema is opaqued to `serde_json::Value`\n(renders as bare `{}` aka JsonValue) so the published schema\ndoesn't inline the entire root union \u2014 that's the TS7056 blowup\nthe root and tier aggregates dodge by being `json_schema_ignore`.\nDownstream SDKs see `value: JsonValue` on the typed `execute`\npath; consumers that want to peer inside parse it case-by-case.").meta({ title: "cli.command.agents.queue.deliver.ValueResponseItem" });

// src/cli/command/agents/queue/deliver/responseItem.ts
var CliCommandAgentsQueueDeliverResponseItemSchema = zod.z.union([CliCommandAgentsQueueDeliverValueResponseItemSchema.meta({ "title": "cli.command.agents.queue.deliver.ValueResponseItem", "variantTitle": "Value" }), CliCommandAgentsQueueDeliverAgentActiveResponseItemSchema.meta({ "title": "cli.command.agents.queue.deliver.AgentActiveResponseItem", "variantTitle": "AgentActive" }), CliCommandAgentsQueueDeliverAgentSpawnedResponseItemSchema.meta({ "title": "cli.command.agents.queue.deliver.AgentSpawnedResponseItem", "variantTitle": "AgentSpawned" }), CliCommandAgentsQueueDeliverTagActiveResponseItemSchema.meta({ "title": "cli.command.agents.queue.deliver.TagActiveResponseItem", "variantTitle": "TagActive" }), CliCommandAgentsQueueDeliverTagSpawnedResponseItemSchema.meta({ "title": "cli.command.agents.queue.deliver.TagSpawnedResponseItem", "variantTitle": "TagSpawned" }), CliCommandAgentsQueueDeliverAllAgentsActiveSchema.meta({ "title": "cli.command.agents.queue.deliver.AllAgentsActive", "variantTitle": "AllAgentsActive" })]).describe('One stream item from `agents queue deliver`. Untagged \u2014 the\nvariants are disjoint on the wire: `Value` requires `value`,\n`AgentActive` / `AgentSpawned` / `TagActive` / `TagSpawned` carry\ndistinct `type` markers, and `AllAgentsActive` is the bare string\n`"AllAgentsActive"`.').meta({ title: "cli.command.agents.queue.deliver.ResponseItem" });

// src/viewer/command/agents/queue/deliver.ts
function agentsQueueDeliverExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/queue/deliver" }), zod.z.union([CliErrorSchema, CliCommandAgentsQueueDeliverResponseItemSchema]));
}
function agentsQueueDeliverExecuteTransform(request, transform) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/queue/deliver" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function agentsQueueDeliverRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/queue/deliver/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue deliver request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsQueueDeliverRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/queue/deliver/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue deliver request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsQueueDeliverResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/queue/deliver/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue deliver response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsQueueDeliverResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/queue/deliver/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue deliver response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsQueueReadIdExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/queue/read/id" }), zod.z.union([CliErrorSchema, AgentCompletionsMessageRichContentPartSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue read id: cli produced no output before the end marker");
  }
  return first;
}
async function agentsQueueReadIdExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/queue/read/id" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue read id: cli produced no output before the end marker");
  }
  return first;
}
async function agentsQueueReadIdRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/queue/read/id/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue read id request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsQueueReadIdRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/queue/read/id/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue read id request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsQueueReadIdResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/queue/read/id/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue read id response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsQueueReadIdResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/queue/read/id/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue read id response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandAgentsQueueReadPendingQueuePartSchema = zod.z.object({
  id: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  type: CliCommandAgentsLogsReadAllClientNotificationPartTypeSchema
}).describe("One row inside a `ResponseItem` block \u2014 a\n`message_queue_contents` entry. The `id` is the\n`message_queue_contents.id`, which you pass to\n`agents queue read id <n>` to drill into the body.\n`enqueued_at` is on the enclosing block, not here\n(one block = one `message_queue` parent row, sharing one\n`enqueued_at`).").meta({ title: "cli.command.agents.queue.read.pending.QueuePart" });

// src/cli/command/agents/queue/read/pending/responseItem.ts
var CliCommandAgentsQueueReadPendingResponseItemSchema = zod.z.union([zod.z.object({
  agent_instance_hierarchy: zod.z.string(),
  by: zod.z.literal("agent_instance_hierarchy"),
  delete_id: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3).describe("`message_queue.id` \u2014 the row-level id this block\nrepresents. Pass to `agents queue delete <id>` to\nsoft-flip the entire row (all parts) in one call.\nDistinct from each `QueuePart.id` (which is a\n`message_queue_contents.id` for drilling into one\ncontent slot via `agents queue read id`)."),
  enqueued_at: zod.z.string().describe("`message_queue.enqueued_at`. One block = one parent\n`message_queue` row, so this is well-defined\nblock-level."),
  key: zod.z.string().nullable().describe("Idempotency token, if the row was enqueued with `--key`.").meta({ omitempty: true }).optional(),
  parts: zod.z.array(CliCommandAgentsQueueReadPendingQueuePartSchema),
  sender_agent_instance_hierarchy: zod.z.string().describe("AIH of the caller who enqueued \u2014 from\n`message_queue.sender_*`.")
}).meta({ "variantTitle": "AgentInstanceHierarchy" }), zod.z.object({
  agent_tag: zod.z.string(),
  by: zod.z.literal("tag"),
  delete_id: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3).describe("`message_queue.id`. Pass to `agents queue delete <id>`."),
  enqueued_at: zod.z.string(),
  key: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  parts: zod.z.array(CliCommandAgentsQueueReadPendingQueuePartSchema),
  sender_agent_instance_hierarchy: zod.z.string()
}).meta({ "variantTitle": "Tag" })]).describe("One pending `message_queue` row, with its content rows\ngrouped as `parts`. Two variants \u2014 direct AIH target or tag\ntarget \u2014 both flat (no nested `LookupState`).").meta({ title: "cli.command.agents.queue.read.pending.ResponseItem" });

// src/viewer/command/agents/queue/read/pending.ts
function agentsQueueReadPendingExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/queue/read/pending" }), zod.z.union([CliErrorSchema, CliCommandAgentsQueueReadPendingResponseItemSchema]));
}
function agentsQueueReadPendingExecuteTransform(request, transform) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/queue/read/pending" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function agentsQueueReadPendingRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/queue/read/pending/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue read pending request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsQueueReadPendingRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/queue/read/pending/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue read pending request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsQueueReadPendingResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/queue/read/pending/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue read pending response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsQueueReadPendingResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/queue/read/pending/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue read pending response_schema: cli produced no output before the end marker");
  }
  return first;
}
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

// src/cli/command/agents/spawn/responseItem.ts
var CliCommandAgentsSpawnResponseItemSchema = zod.z.union([AgentCompletionsResponseStreamingAgentCompletionChunkSchema.meta({ "title": "agent.completions.response.streaming.AgentCompletionChunk", "variantTitle": "Chunk" }), zod.z.string().meta({ "variantTitle": "Id" })]).meta({ title: "cli.command.agents.spawn.ResponseItem" });

// src/viewer/command/agents/spawn.ts
function agentsSpawnExecuteStreaming(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, dangerous_advanced: { ...request.dangerous_advanced ?? {}, stream: true }, path_type: "agents/spawn" }), zod.z.union([CliErrorSchema, CliCommandAgentsSpawnResponseItemSchema]));
}
function agentsSpawnExecuteStreamingTransform(request, transform) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, dangerous_advanced: { ...request.dangerous_advanced ?? {}, stream: true }, path_type: "agents/spawn" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function agentsSpawnExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, dangerous_advanced: request.dangerous_advanced ? { ...request.dangerous_advanced, stream: void 0 } : request.dangerous_advanced, path_type: "agents/spawn" }), zod.z.union([CliErrorSchema, zod.z.string()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents spawn: cli produced no output before the end marker");
  }
  return first;
}
async function agentsSpawnExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, dangerous_advanced: request.dangerous_advanced ? { ...request.dangerous_advanced, stream: void 0 } : request.dangerous_advanced, path_type: "agents/spawn" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents spawn: cli produced no output before the end marker");
  }
  return first;
}
async function agentsSpawnRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/spawn/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents spawn request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsSpawnRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/spawn/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents spawn request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsSpawnResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/spawn/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents spawn response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsSpawnResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/spawn/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents spawn response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandAgentsTagsApplyResponseSchema = zod.z.union([zod.z.object({
  agent_instance: zod.z.string(),
  agent_instance_hierarchy: zod.z.string().describe("`{parent}/{agent_instance}`."),
  by: zod.z.literal("agent_instance"),
  name: zod.z.string(),
  parent_agent_instance_hierarchy: zod.z.string()
}).meta({ "variantTitle": "AgentInstance" }), zod.z.object({
  agent_spec: AgentInlineAgentBaseWithFallbacksOrRemoteCommitOptionalSchema,
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
  agent_spec: AgentInlineAgentBaseWithFallbacksOrRemoteCommitOptionalSchema,
  agent_tag: zod.z.string(),
  by: zod.z.literal("agent_tag"),
  name: zod.z.string(),
  parent_agent_instance_hierarchy: zod.z.string(),
  state: zod.z.literal("grouped"),
  tag_group_id: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3)
}).meta({ "variantTitle": "Grouped" })]).describe('Wire shape mirrors the resolved source state:\n`{"by":"agent_tag","name":...,"agent_tag":...,"state":"bound"|"grouped", \u2026}`.').meta({ "variantTitle": "AgentTag" })]).describe("Apply response. Each variant carries the freshly-applied state\nso callers don't need a follow-up lookup.").meta({ title: "cli.command.agents.tags.apply.Response" });

// src/viewer/command/agents/tags/apply.ts
async function agentsTagsApplyExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/tags/apply" }), zod.z.union([CliErrorSchema, CliCommandAgentsTagsApplyResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tags apply: cli produced no output before the end marker");
  }
  return first;
}
async function agentsTagsApplyExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/tags/apply" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tags apply: cli produced no output before the end marker");
  }
  return first;
}
async function agentsTagsApplyRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/tags/apply/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tags apply request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsTagsApplyRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/tags/apply/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tags apply request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsTagsApplyResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/tags/apply/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tags apply response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsTagsApplyResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/tags/apply/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tags apply response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsTagsLookupRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/tags/lookup/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tags lookup request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsTagsLookupRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/tags/lookup/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tags lookup request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsTagsLookupResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/tags/lookup/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tags lookup response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsTagsLookupResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/tags/lookup/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tags lookup response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandOkSchema = zod.z.literal("Ok").describe('Success-only response shape. Wire form is the bare string `"Ok"` \u2014\na single-variant enum gives us a typed sentinel that serializes and\ndeserializes through serde as the static string. Used as `Response`\non every cli leaf whose only success signal is "it worked."').meta({ title: "cli.command.Ok" });

// src/viewer/command/agents/wait.ts
async function agentsWaitExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/wait" }), zod.z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents wait: cli produced no output before the end marker");
  }
  return first;
}
async function agentsWaitExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/wait" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents wait: cli produced no output before the end marker");
  }
  return first;
}
async function agentsWaitRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/wait/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents wait request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsWaitRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/wait/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents wait request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsWaitResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/wait/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents wait response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsWaitResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/wait/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents wait response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandApiConfigAddressGetResponseSchema = zod.z.object({
  address: zod.z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.api.config.address.get.Response" });

// src/viewer/command/api/config/address/get.ts
async function apiConfigAddressGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/address/get" }), zod.z.union([CliErrorSchema, CliCommandApiConfigAddressGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config address get: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigAddressGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/address/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config address get: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigAddressGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/address/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config address get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigAddressGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/address/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config address get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigAddressGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/address/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config address get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigAddressGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/address/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config address get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigAddressSetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "api/config/address/set" }), zod.z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config address set: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigAddressSetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "api/config/address/set" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config address set: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigAddressSetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/address/set/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config address set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigAddressSetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/address/set/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config address set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigAddressSetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/address/set/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config address set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigAddressSetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/address/set/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config address set response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandApiConfigCommitAuthorEmailGetResponseSchema = zod.z.object({
  commit_author_email: zod.z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.api.config.commit_author_email.get.Response" });

// src/viewer/command/api/config/commit_author_email/get.ts
async function apiConfigCommitAuthorEmailGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/commit_author_email/get" }), zod.z.union([CliErrorSchema, CliCommandApiConfigCommitAuthorEmailGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config commit_author_email get: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigCommitAuthorEmailGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/commit_author_email/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config commit_author_email get: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigCommitAuthorEmailGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/commit_author_email/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config commit_author_email get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigCommitAuthorEmailGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/commit_author_email/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config commit_author_email get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigCommitAuthorEmailGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/commit_author_email/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config commit_author_email get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigCommitAuthorEmailGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/commit_author_email/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config commit_author_email get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigCommitAuthorEmailSetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "api/config/commit_author_email/set" }), zod.z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config commit_author_email set: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigCommitAuthorEmailSetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "api/config/commit_author_email/set" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config commit_author_email set: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigCommitAuthorEmailSetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/commit_author_email/set/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config commit_author_email set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigCommitAuthorEmailSetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/commit_author_email/set/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config commit_author_email set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigCommitAuthorEmailSetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/commit_author_email/set/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config commit_author_email set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigCommitAuthorEmailSetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/commit_author_email/set/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config commit_author_email set response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandApiConfigCommitAuthorNameGetResponseSchema = zod.z.object({
  commit_author_name: zod.z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.api.config.commit_author_name.get.Response" });

// src/viewer/command/api/config/commit_author_name/get.ts
async function apiConfigCommitAuthorNameGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/commit_author_name/get" }), zod.z.union([CliErrorSchema, CliCommandApiConfigCommitAuthorNameGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config commit_author_name get: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigCommitAuthorNameGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/commit_author_name/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config commit_author_name get: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigCommitAuthorNameGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/commit_author_name/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config commit_author_name get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigCommitAuthorNameGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/commit_author_name/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config commit_author_name get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigCommitAuthorNameGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/commit_author_name/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config commit_author_name get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigCommitAuthorNameGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/commit_author_name/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config commit_author_name get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigCommitAuthorNameSetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "api/config/commit_author_name/set" }), zod.z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config commit_author_name set: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigCommitAuthorNameSetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "api/config/commit_author_name/set" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config commit_author_name set: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigCommitAuthorNameSetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/commit_author_name/set/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config commit_author_name set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigCommitAuthorNameSetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/commit_author_name/set/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config commit_author_name set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigCommitAuthorNameSetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/commit_author_name/set/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config commit_author_name set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigCommitAuthorNameSetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/commit_author_name/set/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config commit_author_name set response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandApiConfigGetResponseSchema = zod.z.object({
  address: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  commit_author_email: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  commit_author_name: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  github_authorization: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  http_referer: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  mcp_authorization: zod.z.record(zod.z.string(), zod.z.string()).nullable().meta({ omitempty: true }).optional(),
  objectiveai_authorization: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  openrouter_authorization: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  user_agent: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  x_title: zod.z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.api.config.get.Response" });

// src/viewer/command/api/config/get.ts
async function apiConfigGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/get" }), zod.z.union([CliErrorSchema, CliCommandApiConfigGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config get: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config get: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandApiConfigGithubAuthorizationGetResponseSchema = zod.z.object({
  github_authorization: zod.z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.api.config.github_authorization.get.Response" });

// src/viewer/command/api/config/github_authorization/get.ts
async function apiConfigGithubAuthorizationGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/github_authorization/get" }), zod.z.union([CliErrorSchema, CliCommandApiConfigGithubAuthorizationGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config github_authorization get: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigGithubAuthorizationGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/github_authorization/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config github_authorization get: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigGithubAuthorizationGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/github_authorization/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config github_authorization get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigGithubAuthorizationGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/github_authorization/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config github_authorization get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigGithubAuthorizationGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/github_authorization/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config github_authorization get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigGithubAuthorizationGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/github_authorization/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config github_authorization get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigGithubAuthorizationSetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "api/config/github_authorization/set" }), zod.z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config github_authorization set: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigGithubAuthorizationSetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "api/config/github_authorization/set" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config github_authorization set: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigGithubAuthorizationSetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/github_authorization/set/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config github_authorization set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigGithubAuthorizationSetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/github_authorization/set/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config github_authorization set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigGithubAuthorizationSetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/github_authorization/set/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config github_authorization set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigGithubAuthorizationSetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/github_authorization/set/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config github_authorization set response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandApiConfigHttpRefererGetResponseSchema = zod.z.object({
  http_referer: zod.z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.api.config.http_referer.get.Response" });

// src/viewer/command/api/config/http_referer/get.ts
async function apiConfigHttpRefererGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/http_referer/get" }), zod.z.union([CliErrorSchema, CliCommandApiConfigHttpRefererGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config http_referer get: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigHttpRefererGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/http_referer/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config http_referer get: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigHttpRefererGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/http_referer/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config http_referer get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigHttpRefererGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/http_referer/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config http_referer get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigHttpRefererGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/http_referer/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config http_referer get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigHttpRefererGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/http_referer/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config http_referer get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigHttpRefererSetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "api/config/http_referer/set" }), zod.z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config http_referer set: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigHttpRefererSetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "api/config/http_referer/set" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config http_referer set: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigHttpRefererSetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/http_referer/set/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config http_referer set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigHttpRefererSetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/http_referer/set/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config http_referer set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigHttpRefererSetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/http_referer/set/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config http_referer set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigHttpRefererSetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/http_referer/set/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config http_referer set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigMcpAuthorizationAddExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "api/config/mcp_authorization/add" }), zod.z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config mcp_authorization add: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigMcpAuthorizationAddExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "api/config/mcp_authorization/add" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config mcp_authorization add: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigMcpAuthorizationAddRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/mcp_authorization/add/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config mcp_authorization add request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigMcpAuthorizationAddRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/mcp_authorization/add/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config mcp_authorization add request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigMcpAuthorizationAddResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/mcp_authorization/add/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config mcp_authorization add response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigMcpAuthorizationAddResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/mcp_authorization/add/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config mcp_authorization add response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigMcpAuthorizationDelExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "api/config/mcp_authorization/del" }), zod.z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config mcp_authorization del: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigMcpAuthorizationDelExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "api/config/mcp_authorization/del" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config mcp_authorization del: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigMcpAuthorizationDelRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/mcp_authorization/del/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config mcp_authorization del request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigMcpAuthorizationDelRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/mcp_authorization/del/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config mcp_authorization del request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigMcpAuthorizationDelResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/mcp_authorization/del/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config mcp_authorization del response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigMcpAuthorizationDelResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/mcp_authorization/del/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config mcp_authorization del response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandApiConfigMcpAuthorizationGetResponseSchema = zod.z.object({
  mcp_authorization: zod.z.record(zod.z.string(), zod.z.string()).nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.api.config.mcp_authorization.get.Response" });

// src/viewer/command/api/config/mcp_authorization/get.ts
async function apiConfigMcpAuthorizationGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/mcp_authorization/get" }), zod.z.union([CliErrorSchema, CliCommandApiConfigMcpAuthorizationGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config mcp_authorization get: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigMcpAuthorizationGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/mcp_authorization/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config mcp_authorization get: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigMcpAuthorizationGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/mcp_authorization/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config mcp_authorization get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigMcpAuthorizationGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/mcp_authorization/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config mcp_authorization get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigMcpAuthorizationGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/mcp_authorization/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config mcp_authorization get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigMcpAuthorizationGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/mcp_authorization/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config mcp_authorization get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandApiConfigObjectiveaiAuthorizationGetResponseSchema = zod.z.object({
  objectiveai_authorization: zod.z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.api.config.objectiveai_authorization.get.Response" });

// src/viewer/command/api/config/objectiveai_authorization/get.ts
async function apiConfigObjectiveaiAuthorizationGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/objectiveai_authorization/get" }), zod.z.union([CliErrorSchema, CliCommandApiConfigObjectiveaiAuthorizationGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config objectiveai_authorization get: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigObjectiveaiAuthorizationGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/objectiveai_authorization/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config objectiveai_authorization get: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigObjectiveaiAuthorizationGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/objectiveai_authorization/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config objectiveai_authorization get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigObjectiveaiAuthorizationGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/objectiveai_authorization/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config objectiveai_authorization get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigObjectiveaiAuthorizationGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/objectiveai_authorization/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config objectiveai_authorization get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigObjectiveaiAuthorizationGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/objectiveai_authorization/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config objectiveai_authorization get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigObjectiveaiAuthorizationSetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "api/config/objectiveai_authorization/set" }), zod.z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config objectiveai_authorization set: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigObjectiveaiAuthorizationSetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "api/config/objectiveai_authorization/set" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config objectiveai_authorization set: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigObjectiveaiAuthorizationSetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/objectiveai_authorization/set/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config objectiveai_authorization set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigObjectiveaiAuthorizationSetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/objectiveai_authorization/set/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config objectiveai_authorization set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigObjectiveaiAuthorizationSetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/objectiveai_authorization/set/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config objectiveai_authorization set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigObjectiveaiAuthorizationSetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/objectiveai_authorization/set/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config objectiveai_authorization set response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandApiConfigOpenrouterAuthorizationGetResponseSchema = zod.z.object({
  openrouter_authorization: zod.z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.api.config.openrouter_authorization.get.Response" });

// src/viewer/command/api/config/openrouter_authorization/get.ts
async function apiConfigOpenrouterAuthorizationGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/openrouter_authorization/get" }), zod.z.union([CliErrorSchema, CliCommandApiConfigOpenrouterAuthorizationGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config openrouter_authorization get: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigOpenrouterAuthorizationGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/openrouter_authorization/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config openrouter_authorization get: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigOpenrouterAuthorizationGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/openrouter_authorization/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config openrouter_authorization get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigOpenrouterAuthorizationGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/openrouter_authorization/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config openrouter_authorization get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigOpenrouterAuthorizationGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/openrouter_authorization/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config openrouter_authorization get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigOpenrouterAuthorizationGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/openrouter_authorization/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config openrouter_authorization get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigOpenrouterAuthorizationSetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "api/config/openrouter_authorization/set" }), zod.z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config openrouter_authorization set: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigOpenrouterAuthorizationSetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "api/config/openrouter_authorization/set" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config openrouter_authorization set: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigOpenrouterAuthorizationSetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/openrouter_authorization/set/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config openrouter_authorization set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigOpenrouterAuthorizationSetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/openrouter_authorization/set/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config openrouter_authorization set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigOpenrouterAuthorizationSetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/openrouter_authorization/set/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config openrouter_authorization set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigOpenrouterAuthorizationSetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/openrouter_authorization/set/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config openrouter_authorization set response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandApiConfigUserAgentGetResponseSchema = zod.z.object({
  user_agent: zod.z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.api.config.user_agent.get.Response" });

// src/viewer/command/api/config/user_agent/get.ts
async function apiConfigUserAgentGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/user_agent/get" }), zod.z.union([CliErrorSchema, CliCommandApiConfigUserAgentGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config user_agent get: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigUserAgentGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/user_agent/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config user_agent get: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigUserAgentGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/user_agent/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config user_agent get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigUserAgentGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/user_agent/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config user_agent get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigUserAgentGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/user_agent/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config user_agent get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigUserAgentGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/user_agent/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config user_agent get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigUserAgentSetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "api/config/user_agent/set" }), zod.z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config user_agent set: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigUserAgentSetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "api/config/user_agent/set" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config user_agent set: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigUserAgentSetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/user_agent/set/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config user_agent set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigUserAgentSetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/user_agent/set/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config user_agent set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigUserAgentSetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/user_agent/set/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config user_agent set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigUserAgentSetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/user_agent/set/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config user_agent set response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandApiConfigXTitleGetResponseSchema = zod.z.object({
  x_title: zod.z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.api.config.x_title.get.Response" });

// src/viewer/command/api/config/x_title/get.ts
async function apiConfigXTitleGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/x_title/get" }), zod.z.union([CliErrorSchema, CliCommandApiConfigXTitleGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config x_title get: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigXTitleGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/x_title/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config x_title get: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigXTitleGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/x_title/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config x_title get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigXTitleGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/x_title/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config x_title get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigXTitleGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/x_title/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config x_title get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigXTitleGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/x_title/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config x_title get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigXTitleSetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "api/config/x_title/set" }), zod.z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config x_title set: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigXTitleSetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "api/config/x_title/set" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config x_title set: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigXTitleSetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/x_title/set/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config x_title set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigXTitleSetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/x_title/set/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config x_title set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigXTitleSetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/x_title/set/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config x_title set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigXTitleSetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/x_title/set/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config x_title set response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandApiKillResponseSchema = zod.z.object({
  killed: zod.z.number().int().min(0).max(4294967295)
}).meta({ title: "cli.command.api.kill.Response" });

// src/viewer/command/api/kill.ts
async function apiKillExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/kill" }), zod.z.union([CliErrorSchema, CliCommandApiKillResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api kill: cli produced no output before the end marker");
  }
  return first;
}
async function apiKillExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/kill" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api kill: cli produced no output before the end marker");
  }
  return first;
}
async function apiKillRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/kill/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api kill request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiKillRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/kill/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api kill request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiKillResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/kill/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api kill response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiKillResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/kill/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api kill response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandApiSpawnResponseSchema = zod.z.object({
  listening: zod.z.string()
}).meta({ title: "cli.command.api.spawn.Response" });

// src/viewer/command/api/spawn.ts
async function apiSpawnExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/spawn" }), zod.z.union([CliErrorSchema, CliCommandApiSpawnResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api spawn: cli produced no output before the end marker");
  }
  return first;
}
async function apiSpawnExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/spawn" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api spawn: cli produced no output before the end marker");
  }
  return first;
}
async function apiSpawnRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/spawn/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api spawn request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiSpawnRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/spawn/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api spawn request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiSpawnResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/spawn/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api spawn response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiSpawnResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/spawn/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api spawn response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandDbConfigAddressGetResponseSchema = zod.z.object({
  address: zod.z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.db.config.address.get.Response" });

// src/viewer/command/db/config/address/get.ts
async function dbConfigAddressGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/config/address/get" }), zod.z.union([CliErrorSchema, CliCommandDbConfigAddressGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config address get: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigAddressGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/config/address/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config address get: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigAddressGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/config/address/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config address get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigAddressGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/config/address/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config address get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigAddressGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/config/address/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config address get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigAddressGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/config/address/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config address get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigAddressSetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "db/config/address/set" }), zod.z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config address set: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigAddressSetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "db/config/address/set" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config address set: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigAddressSetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/config/address/set/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config address set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigAddressSetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/config/address/set/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config address set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigAddressSetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/config/address/set/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config address set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigAddressSetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/config/address/set/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config address set response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandDbConfigDatabaseGetResponseSchema = zod.z.object({
  database: zod.z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.db.config.database.get.Response" });

// src/viewer/command/db/config/database/get.ts
async function dbConfigDatabaseGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/config/database/get" }), zod.z.union([CliErrorSchema, CliCommandDbConfigDatabaseGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config database get: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigDatabaseGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/config/database/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config database get: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigDatabaseGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/config/database/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config database get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigDatabaseGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/config/database/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config database get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigDatabaseGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/config/database/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config database get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigDatabaseGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/config/database/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config database get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigDatabaseSetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "db/config/database/set" }), zod.z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config database set: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigDatabaseSetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "db/config/database/set" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config database set: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigDatabaseSetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/config/database/set/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config database set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigDatabaseSetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/config/database/set/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config database set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigDatabaseSetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/config/database/set/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config database set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigDatabaseSetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/config/database/set/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config database set response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandDbConfigGetResponseSchema = zod.z.object({
  address: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  database: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  password: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  user: zod.z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.db.config.get.Response" });

// src/viewer/command/db/config/get.ts
async function dbConfigGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/config/get" }), zod.z.union([CliErrorSchema, CliCommandDbConfigGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config get: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/config/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config get: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/config/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/config/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/config/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/config/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandDbConfigPasswordGetResponseSchema = zod.z.object({
  password: zod.z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.db.config.password.get.Response" });

// src/viewer/command/db/config/password/get.ts
async function dbConfigPasswordGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/config/password/get" }), zod.z.union([CliErrorSchema, CliCommandDbConfigPasswordGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config password get: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigPasswordGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/config/password/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config password get: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigPasswordGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/config/password/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config password get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigPasswordGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/config/password/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config password get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigPasswordGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/config/password/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config password get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigPasswordGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/config/password/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config password get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigPasswordSetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "db/config/password/set" }), zod.z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config password set: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigPasswordSetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "db/config/password/set" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config password set: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigPasswordSetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/config/password/set/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config password set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigPasswordSetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/config/password/set/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config password set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigPasswordSetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/config/password/set/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config password set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigPasswordSetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/config/password/set/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config password set response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandDbConfigUserGetResponseSchema = zod.z.object({
  user: zod.z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.db.config.user.get.Response" });

// src/viewer/command/db/config/user/get.ts
async function dbConfigUserGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/config/user/get" }), zod.z.union([CliErrorSchema, CliCommandDbConfigUserGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config user get: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigUserGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/config/user/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config user get: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigUserGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/config/user/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config user get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigUserGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/config/user/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config user get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigUserGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/config/user/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config user get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigUserGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/config/user/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config user get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigUserSetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "db/config/user/set" }), zod.z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config user set: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigUserSetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "db/config/user/set" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config user set: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigUserSetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/config/user/set/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config user set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigUserSetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/config/user/set/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config user set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigUserSetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/config/user/set/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config user set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigUserSetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/config/user/set/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config user set response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandDbKillResponseSchema = zod.z.object({
  killed: zod.z.number().int().min(0).max(4294967295)
}).meta({ title: "cli.command.db.kill.Response" });

// src/viewer/command/db/kill.ts
async function dbKillExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/kill" }), zod.z.union([CliErrorSchema, CliCommandDbKillResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db kill: cli produced no output before the end marker");
  }
  return first;
}
async function dbKillExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/kill" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db kill: cli produced no output before the end marker");
  }
  return first;
}
async function dbKillRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/kill/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db kill request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbKillRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/kill/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db kill request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbKillResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/kill/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db kill response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbKillResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/kill/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db kill response_schema: cli produced no output before the end marker");
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
  truncated: zod.z.boolean().describe('Always `false` in the current design. Reserved on the wire\nso a future "soft truncation" mode can be added without a\nshape break.')
}).describe("Unary response from `db query`.").meta({ title: "cli.command.db.query.Response" });

// src/viewer/command/db/query.ts
async function dbQueryExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/query" }), zod.z.union([CliErrorSchema, CliCommandDbQueryResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db query: cli produced no output before the end marker");
  }
  return first;
}
async function dbQueryExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/query" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db query: cli produced no output before the end marker");
  }
  return first;
}
async function dbQueryRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/query/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db query request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbQueryRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/query/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db query request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbQueryResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/query/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db query response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbQueryResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/query/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db query response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandDbSpawnResponseSchema = zod.z.object({
  listening: zod.z.string()
}).meta({ title: "cli.command.db.spawn.Response" });

// src/viewer/command/db/spawn.ts
async function dbSpawnExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/spawn" }), zod.z.union([CliErrorSchema, CliCommandDbSpawnResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db spawn: cli produced no output before the end marker");
  }
  return first;
}
async function dbSpawnExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/spawn" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db spawn: cli produced no output before the end marker");
  }
  return first;
}
async function dbSpawnRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/spawn/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db spawn request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbSpawnRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/spawn/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db spawn request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbSpawnResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/spawn/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db spawn response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbSpawnResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/spawn/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db spawn response_schema: cli produced no output before the end marker");
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
  prompt_id: zod.z.string().describe("Content hash of the request messages (for caching/deduplication)."),
  responses_ids: zod.z.array(zod.z.string()).describe("Content hashes of each response option in the request."),
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
  tasks: zod.z.array(FunctionsExecutionsResponseStreamingTaskChunkSchema),
  tasks_errors: zod.z.boolean().nullable().meta({ omitempty: true }).optional(),
  usage: AgentCompletionsResponseUsageSchema.nullable().meta({ omitempty: true }).optional()
}).meta({ title: "functions.executions.response.streaming.FunctionExecutionChunk" });

// src/cli/command/functions/execute/standard/responseItem.ts
var CliCommandFunctionsExecuteStandardResponseItemSchema = zod.z.union([FunctionsExecutionsResponseStreamingFunctionExecutionChunkSchema.meta({ "title": "functions.executions.response.streaming.FunctionExecutionChunk", "variantTitle": "Chunk" }), zod.z.string().meta({ "variantTitle": "Id" })]).meta({ title: "cli.command.functions.execute.standard.ResponseItem" });

// src/viewer/command/functions/execute/standard.ts
function functionsExecuteStandardExecuteStreaming(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, dangerous_advanced: { ...request.dangerous_advanced ?? {}, stream: true }, path_type: "functions/execute/standard" }), zod.z.union([CliErrorSchema, CliCommandFunctionsExecuteStandardResponseItemSchema]));
}
function functionsExecuteStandardExecuteStreamingTransform(request, transform) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, dangerous_advanced: { ...request.dangerous_advanced ?? {}, stream: true }, path_type: "functions/execute/standard" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function functionsExecuteStandardExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, dangerous_advanced: request.dangerous_advanced ? { ...request.dangerous_advanced, stream: void 0 } : request.dangerous_advanced, path_type: "functions/execute/standard" }), zod.z.union([CliErrorSchema, zod.z.string()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions execute standard: cli produced no output before the end marker");
  }
  return first;
}
async function functionsExecuteStandardExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, dangerous_advanced: request.dangerous_advanced ? { ...request.dangerous_advanced, stream: void 0 } : request.dangerous_advanced, path_type: "functions/execute/standard" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions execute standard: cli produced no output before the end marker");
  }
  return first;
}
async function functionsExecuteStandardRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "functions/execute/standard/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions execute standard request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsExecuteStandardRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "functions/execute/standard/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions execute standard request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsExecuteStandardResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "functions/execute/standard/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions execute standard response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsExecuteStandardResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "functions/execute/standard/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions execute standard response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandFunctionsExecuteSwissSystemResponseItemSchema = zod.z.union([FunctionsExecutionsResponseStreamingFunctionExecutionChunkSchema.meta({ "title": "functions.executions.response.streaming.FunctionExecutionChunk", "variantTitle": "Chunk" }), zod.z.string().meta({ "variantTitle": "Id" })]).meta({ title: "cli.command.functions.execute.swiss_system.ResponseItem" });

// src/viewer/command/functions/execute/swiss_system.ts
function functionsExecuteSwissSystemExecuteStreaming(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, dangerous_advanced: { ...request.dangerous_advanced ?? {}, stream: true }, path_type: "functions/execute/swiss_system" }), zod.z.union([CliErrorSchema, CliCommandFunctionsExecuteSwissSystemResponseItemSchema]));
}
function functionsExecuteSwissSystemExecuteStreamingTransform(request, transform) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, dangerous_advanced: { ...request.dangerous_advanced ?? {}, stream: true }, path_type: "functions/execute/swiss_system" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function functionsExecuteSwissSystemExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, dangerous_advanced: request.dangerous_advanced ? { ...request.dangerous_advanced, stream: void 0 } : request.dangerous_advanced, path_type: "functions/execute/swiss_system" }), zod.z.union([CliErrorSchema, zod.z.string()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions execute swiss_system: cli produced no output before the end marker");
  }
  return first;
}
async function functionsExecuteSwissSystemExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, dangerous_advanced: request.dangerous_advanced ? { ...request.dangerous_advanced, stream: void 0 } : request.dangerous_advanced, path_type: "functions/execute/swiss_system" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions execute swiss_system: cli produced no output before the end marker");
  }
  return first;
}
async function functionsExecuteSwissSystemRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "functions/execute/swiss_system/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions execute swiss_system request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsExecuteSwissSystemRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "functions/execute/swiss_system/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions execute swiss_system request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsExecuteSwissSystemResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "functions/execute/swiss_system/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions execute swiss_system response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsExecuteSwissSystemResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "functions/execute/swiss_system/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions execute swiss_system response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandFunctionsGetResponseSchema = RemotePathSchema.and(zod.z.object({})).describe("Response for `functions get` \u2014 a resolved Function with its remote path.").meta({ title: "cli.command.functions.get.Response" });

// src/viewer/command/functions/get.ts
async function functionsGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "functions/get" }), zod.z.union([CliErrorSchema, CliCommandFunctionsGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions get: cli produced no output before the end marker");
  }
  return first;
}
async function functionsGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "functions/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions get: cli produced no output before the end marker");
  }
  return first;
}
async function functionsGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "functions/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "functions/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "functions/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "functions/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions get response_schema: cli produced no output before the end marker");
  }
  return first;
}
function functionsListExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "functions/list" }), zod.z.union([CliErrorSchema, RemotePathSchema]));
}
function functionsListExecuteTransform(request, transform) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "functions/list" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function functionsListRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "functions/list/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsListRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "functions/list/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsListResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "functions/list/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions list response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsListResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "functions/list/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions list response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandFunctionsProfilesGetResponseSchema = RemotePathSchema.and(zod.z.object({})).describe("Response for `functions profiles get` \u2014 a resolved Profile with its remote path.").meta({ title: "cli.command.functions.profiles.get.Response" });

// src/viewer/command/functions/profiles/get.ts
async function functionsProfilesGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "functions/profiles/get" }), zod.z.union([CliErrorSchema, CliCommandFunctionsProfilesGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles get: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "functions/profiles/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles get: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "functions/profiles/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "functions/profiles/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "functions/profiles/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "functions/profiles/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles get response_schema: cli produced no output before the end marker");
  }
  return first;
}
function functionsProfilesListExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "functions/profiles/list" }), zod.z.union([CliErrorSchema, RemotePathSchema]));
}
function functionsProfilesListExecuteTransform(request, transform) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "functions/profiles/list" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function functionsProfilesListRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "functions/profiles/list/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesListRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "functions/profiles/list/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesListResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "functions/profiles/list/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles list response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesListResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "functions/profiles/list/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
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
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "functions/profiles/publish" }), zod.z.union([CliErrorSchema, CliCommandFunctionsProfilesPublishResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles publish: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesPublishExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "functions/profiles/publish" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles publish: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesPublishRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "functions/profiles/publish/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles publish request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesPublishRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "functions/profiles/publish/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles publish request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesPublishResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "functions/profiles/publish/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles publish response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesPublishResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "functions/profiles/publish/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
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
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "functions/publish" }), zod.z.union([CliErrorSchema, CliCommandFunctionsPublishResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions publish: cli produced no output before the end marker");
  }
  return first;
}
async function functionsPublishExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "functions/publish" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions publish: cli produced no output before the end marker");
  }
  return first;
}
async function functionsPublishRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "functions/publish/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions publish request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsPublishRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "functions/publish/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions publish request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsPublishResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "functions/publish/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions publish response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsPublishResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "functions/publish/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions publish response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandMcpConfigAddressGetResponseSchema = zod.z.object({
  address: zod.z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.mcp.config.address.get.Response" });

// src/viewer/command/mcp/config/address/get.ts
async function mcpConfigAddressGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "mcp/config/address/get" }), zod.z.union([CliErrorSchema, CliCommandMcpConfigAddressGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config address get: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigAddressGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "mcp/config/address/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config address get: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigAddressGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "mcp/config/address/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config address get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigAddressGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "mcp/config/address/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config address get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigAddressGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "mcp/config/address/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config address get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigAddressGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "mcp/config/address/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config address get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigAddressSetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "mcp/config/address/set" }), zod.z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config address set: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigAddressSetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "mcp/config/address/set" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config address set: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigAddressSetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "mcp/config/address/set/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config address set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigAddressSetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "mcp/config/address/set/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config address set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigAddressSetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "mcp/config/address/set/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config address set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigAddressSetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "mcp/config/address/set/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config address set response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandMcpConfigGetResponseSchema = zod.z.object({
  address: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  port: zod.z.number().int().min(0).max(65535).nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.mcp.config.get.Response" });

// src/viewer/command/mcp/config/get.ts
async function mcpConfigGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "mcp/config/get" }), zod.z.union([CliErrorSchema, CliCommandMcpConfigGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config get: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "mcp/config/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config get: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "mcp/config/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "mcp/config/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "mcp/config/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "mcp/config/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandMcpConfigPortGetResponseSchema = zod.z.object({
  port: zod.z.number().int().min(0).max(65535)
}).meta({ title: "cli.command.mcp.config.port.get.Response" });

// src/viewer/command/mcp/config/port/get.ts
async function mcpConfigPortGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "mcp/config/port/get" }), zod.z.union([CliErrorSchema, CliCommandMcpConfigPortGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config port get: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigPortGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "mcp/config/port/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config port get: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigPortGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "mcp/config/port/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config port get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigPortGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "mcp/config/port/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config port get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigPortGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "mcp/config/port/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config port get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigPortGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "mcp/config/port/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config port get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigPortSetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "mcp/config/port/set" }), zod.z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config port set: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigPortSetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "mcp/config/port/set" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config port set: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigPortSetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "mcp/config/port/set/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config port set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigPortSetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "mcp/config/port/set/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config port set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigPortSetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "mcp/config/port/set/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config port set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigPortSetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "mcp/config/port/set/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config port set response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandMcpKillResponseSchema = zod.z.object({
  killed: zod.z.number().int().min(0).max(4294967295)
}).meta({ title: "cli.command.mcp.kill.Response" });

// src/viewer/command/mcp/kill.ts
async function mcpKillExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "mcp/kill" }), zod.z.union([CliErrorSchema, CliCommandMcpKillResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp kill: cli produced no output before the end marker");
  }
  return first;
}
async function mcpKillExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "mcp/kill" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp kill: cli produced no output before the end marker");
  }
  return first;
}
async function mcpKillRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "mcp/kill/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp kill request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpKillRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "mcp/kill/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp kill request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpKillResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "mcp/kill/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp kill response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpKillResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "mcp/kill/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
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
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "mcp/spawn" }), zod.z.union([CliErrorSchema, CliCommandMcpSpawnResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp spawn: cli produced no output before the end marker");
  }
  return first;
}
async function mcpSpawnExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "mcp/spawn" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp spawn: cli produced no output before the end marker");
  }
  return first;
}
async function mcpSpawnRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "mcp/spawn/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp spawn request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpSpawnRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "mcp/spawn/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp spawn request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpSpawnResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "mcp/spawn/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp spawn response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpSpawnResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "mcp/spawn/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp spawn response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandPluginsGetResponseMcpServerSchema = zod.z.object({
  authorization: zod.z.boolean(),
  name: zod.z.string()
}).meta({ title: "cli.command.plugins.get.ResponseMcpServer" });
var CliCommandPluginsGetResponseHttpMethodSchema = zod.z.enum(["GET", "POST", "PUT", "PATCH", "DELETE"]).meta({ title: "cli.command.plugins.get.ResponseHttpMethod" });

// src/cli/command/plugins/get/responseViewerRoute.ts
var CliCommandPluginsGetResponseViewerRouteSchema = zod.z.object({
  method: CliCommandPluginsGetResponseHttpMethodSchema,
  path: zod.z.string(),
  type: zod.z.string()
}).meta({ title: "cli.command.plugins.get.ResponseViewerRoute" });
var CliCommandToolsGetExecSchema = zod.z.object({
  linux: zod.z.array(zod.z.string()),
  macos: zod.z.array(zod.z.string()),
  windows: zod.z.array(zod.z.string())
}).describe("Per-OS exec command for a tool. The current platform's vector is\nthe program plus its leading arguments; the caller's `--args` are\nappended, and the result runs with CWD = the tool's version folder's\n`cli/` subdir (`objectiveai.json` lives in the version folder).").meta({ title: "cli.command.tools.get.Exec" });

// src/cli/command/plugins/get/responseManifest.ts
var CliCommandPluginsGetResponseManifestSchema = zod.z.object({
  description: zod.z.string(),
  exec: CliCommandToolsGetExecSchema.describe("Per-OS exec argv for the plugin's cli side, run with CWD =\n`<plugin dir>/cli/` \u2014 the same shape tools use. Required (an\nempty exec is a viewer-only plugin, which still round-trips as\nan empty per-OS object)."),
  mcp_servers: zod.z.array(CliCommandPluginsGetResponseMcpServerSchema),
  name: zod.z.string(),
  owner: zod.z.string(),
  version: zod.z.string(),
  viewer_routes: zod.z.array(CliCommandPluginsGetResponseViewerRouteSchema),
  viewer_url: zod.z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.plugins.get.ResponseManifest" });

// src/viewer/command/plugins/get.ts
async function pluginsGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "plugins/get" }), zod.z.union([CliErrorSchema, CliCommandPluginsGetResponseManifestSchema.nullable()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins get: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "plugins/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins get: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "plugins/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "plugins/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "plugins/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "plugins/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
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
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "plugins/install/filesystem" }), zod.z.union([CliErrorSchema, CliCommandPluginsInstallFilesystemResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins install filesystem: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsInstallFilesystemExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "plugins/install/filesystem" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins install filesystem: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsInstallFilesystemRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "plugins/install/filesystem/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins install filesystem request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsInstallFilesystemRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "plugins/install/filesystem/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins install filesystem request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsInstallFilesystemResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "plugins/install/filesystem/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins install filesystem response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsInstallFilesystemResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "plugins/install/filesystem/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
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
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "plugins/install/github" }), zod.z.union([CliErrorSchema, CliCommandPluginsInstallGithubResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins install github: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsInstallGithubExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "plugins/install/github" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins install github: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsInstallGithubRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "plugins/install/github/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins install github request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsInstallGithubRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "plugins/install/github/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins install github request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsInstallGithubResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "plugins/install/github/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins install github response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsInstallGithubResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "plugins/install/github/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins install github response_schema: cli produced no output before the end marker");
  }
  return first;
}
function pluginsListExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "plugins/list" }), zod.z.union([CliErrorSchema, CliCommandPluginsGetResponseManifestSchema]));
}
function pluginsListExecuteTransform(request, transform) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "plugins/list" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function pluginsListRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "plugins/list/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsListRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "plugins/list/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsListResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "plugins/list/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins list response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsListResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "plugins/list/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
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
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "plugins/run" }), zod.z.union([CliErrorSchema, CliCommandPluginsRunResponseItemSchema]));
}
function pluginsRunExecuteTransform(request, transform) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "plugins/run" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function pluginsRunRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "plugins/run/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins run request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsRunRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "plugins/run/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins run request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsRunResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "plugins/run/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins run response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsRunResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "plugins/run/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins run response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandSwarmsGetResponseSchema = RemotePathSchema.and(zod.z.object({})).describe("Response for `swarms get` \u2014 a Swarm base definition with its remote path.").meta({ title: "cli.command.swarms.get.Response" });

// src/viewer/command/swarms/get.ts
async function swarmsGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "swarms/get" }), zod.z.union([CliErrorSchema, CliCommandSwarmsGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms get: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "swarms/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms get: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "swarms/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "swarms/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "swarms/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "swarms/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms get response_schema: cli produced no output before the end marker");
  }
  return first;
}
function swarmsListExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "swarms/list" }), zod.z.union([CliErrorSchema, RemotePathSchema]));
}
function swarmsListExecuteTransform(request, transform) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "swarms/list" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function swarmsListRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "swarms/list/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsListRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "swarms/list/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsListResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "swarms/list/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms list response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsListResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "swarms/list/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
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
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "swarms/publish" }), zod.z.union([CliErrorSchema, CliCommandSwarmsPublishResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms publish: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsPublishExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "swarms/publish" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms publish: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsPublishRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "swarms/publish/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms publish request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsPublishRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "swarms/publish/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms publish request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsPublishResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "swarms/publish/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms publish response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsPublishResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "swarms/publish/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
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
  created_at: zod.z.string().describe("RFC3339 timestamp this schedule row was created."),
  description: zod.z.string(),
  id: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3).describe("The `schedules` row id. Monotonic; pass the highest `id` from a\npage as the next request's `after_id` to paginate forward."),
  interval: zod.z.string().nullable().describe('`None` for a oneshot; `Some("30s" / "1h" / "1d12h" / \u2026)`\nfor a recurring schedule, formatted as humantime so the\nlist output reads naturally without a unit-conversion\nstep at the consumer. The CLI parser accepts the same\nshape on `agents tasks schedule --interval`.').meta({ omitempty: true }).optional(),
  last_ran_at: zod.z.string().nullable().describe("RFC3339 timestamp of the most recent invocation \u2014 this row's\nnewest `tasks_runs` entry. `None` until the runner has fired\nthis version at least once (runs are tracked per-version).").meta({ omitempty: true }).optional(),
  name: zod.z.string().describe("The `--name` passed to `agents tasks schedule`. Unique per\n`agent_instance_hierarchy`."),
  plugin: CliCommandTasksListPluginSchema.nullable().describe("The plugin that registered this schedule (its `(owner,\nrepository, version)` coordinate), or `None` when it was not\nscheduled by a plugin.").meta({ omitempty: true }).optional(),
  version: zod.z.number().int().min(0).max(18446744073709552e3).describe("This row's version: `1` for a freshly scheduled task,\n`max + 1` for each `tasks schedule --overwrite` (each version\nis its own row; only the newest lists).")
}).describe("One schedule row.").meta({ title: "cli.command.tasks.list.ResponseItem" });

// src/viewer/command/tasks/list.ts
function tasksListExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "tasks/list" }), zod.z.union([CliErrorSchema, CliCommandTasksListResponseItemSchema]));
}
function tasksListExecuteTransform(request, transform) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "tasks/list" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function tasksListRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "tasks/list/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tasks list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function tasksListRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "tasks/list/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tasks list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function tasksListResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "tasks/list/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tasks list response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function tasksListResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "tasks/list/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
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
  value: JsonValueSchema.describe("What the scheduled command emitted \u2014 either the typed root item\n(no transform) or post-transform JSON. See [`RunValue`]. Schema\nis opaqued to `serde_json::Value`."),
  version: zod.z.number().int().min(0).max(18446744073709552e3).describe("The source schedule's version (`1` on first creation,\nincremented per `schedule --overwrite`).")
}).describe("One output item from one fired schedule's in-process stream\n(`stream_all` mode). The first four fields identify the source\nschedule; `value` is the typed root\n[`crate::cli::command::ResponseItem`] emitted by the scheduled cli\nleaf \u2014 boxed because the root union transitively contains *this*\ntype (`agents \u2192 tasks \u2192 run`), and boxing is what makes the\nrecursion sized.\n\nThe `value` field's JSON schema is opaqued to `serde_json::Value`\n(renders as bare `{}` aka JsonValue) so the published schema\ndoesn't inline the entire root union \u2014 that's the TS7056 blowup\nthe root and tier aggregates dodge by being `json_schema_ignore`.\nDownstream SDKs see `value: JsonValue` on the typed `execute`\npath; consumers that want to peer inside parse it case-by-case.").meta({ title: "cli.command.tasks.run.ValueResponseItem" });

// src/cli/command/tasks/run/responseItem.ts
var CliCommandTasksRunResponseItemSchema = zod.z.union([CliCommandTasksRunValueResponseItemSchema.meta({ "title": "cli.command.tasks.run.ValueResponseItem", "variantTitle": "Value" }), CliCommandTasksRunSuccessResponseItemSchema.meta({ "title": "cli.command.tasks.run.SuccessResponseItem", "variantTitle": "Success" })]).describe("One stream item from `tasks run`. Untagged \u2014 the variants'\nrequired fields (`value` vs `success`) are disjoint, so the wire\nshape is just the inner object. Which variant flows is decided by\nthe request's `stream_all`: `true` streams every emitted item as a\n[`ValueResponseItem`] (whose `value` is the typed root item for a\nno-transform command, or the post-transform JSON otherwise \u2014 see\n[`RunValue`]); `false` (default) yields exactly one\n[`SuccessResponseItem`] per task when its stream completes.").meta({ title: "cli.command.tasks.run.ResponseItem" });

// src/viewer/command/tasks/run.ts
function tasksRunExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "tasks/run" }), zod.z.union([CliErrorSchema, CliCommandTasksRunResponseItemSchema]));
}
function tasksRunExecuteTransform(request, transform) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "tasks/run" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function tasksRunRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "tasks/run/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tasks run request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function tasksRunRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "tasks/run/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tasks run request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function tasksRunResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "tasks/run/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tasks run response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function tasksRunResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "tasks/run/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
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
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "tasks/schedule" }), zod.z.union([CliErrorSchema, CliCommandTasksScheduleResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tasks schedule: cli produced no output before the end marker");
  }
  return first;
}
async function tasksScheduleExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "tasks/schedule" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tasks schedule: cli produced no output before the end marker");
  }
  return first;
}
async function tasksScheduleRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "tasks/schedule/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tasks schedule request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function tasksScheduleRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "tasks/schedule/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tasks schedule request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function tasksScheduleResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "tasks/schedule/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tasks schedule response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function tasksScheduleResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "tasks/schedule/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tasks schedule response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandToolsGetResponseManifestSchema = zod.z.object({
  description: zod.z.string(),
  exec: CliCommandToolsGetExecSchema,
  name: zod.z.string(),
  owner: zod.z.string(),
  version: zod.z.string()
}).describe("Wire response for `tools get` \u2014 a lean projection of the on-disk\nmanifest. `exec` is required (a tool always has a command). The\non-disk-only fields (`cli_zip`, `source`) are intentionally absent;\nthe CLI owns the full on-disk shape in its own `filesystem::tools`\nmanifest types.").meta({ title: "cli.command.tools.get.ResponseManifest" });

// src/viewer/command/tools/get.ts
async function toolsGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "tools/get" }), zod.z.union([CliErrorSchema, CliCommandToolsGetResponseManifestSchema.nullable()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools get: cli produced no output before the end marker");
  }
  return first;
}
async function toolsGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "tools/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools get: cli produced no output before the end marker");
  }
  return first;
}
async function toolsGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "tools/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function toolsGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "tools/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function toolsGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "tools/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function toolsGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "tools/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
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
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "tools/install" }), zod.z.union([CliErrorSchema, CliCommandToolsInstallResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools install: cli produced no output before the end marker");
  }
  return first;
}
async function toolsInstallExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "tools/install" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools install: cli produced no output before the end marker");
  }
  return first;
}
async function toolsInstallRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "tools/install/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools install request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function toolsInstallRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "tools/install/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools install request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function toolsInstallResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "tools/install/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools install response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function toolsInstallResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "tools/install/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools install response_schema: cli produced no output before the end marker");
  }
  return first;
}
function toolsListExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "tools/list" }), zod.z.union([CliErrorSchema, CliCommandToolsGetResponseManifestSchema]));
}
function toolsListExecuteTransform(request, transform) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "tools/list" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function toolsListRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "tools/list/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function toolsListRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "tools/list/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function toolsListResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "tools/list/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools list response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function toolsListResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "tools/list/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools list response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandToolsRunResponseItemSchema = zod.z.union([zod.z.string().meta({ "variantTitle": "Stdout" }), CliErrorSchema.meta({ "title": "cli.Error", "variantTitle": "Stderr" })]).meta({ title: "cli.command.tools.run.ResponseItem" });

// src/viewer/command/tools/run.ts
function toolsRunExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "tools/run" }), zod.z.union([CliErrorSchema, CliCommandToolsRunResponseItemSchema]));
}
function toolsRunExecuteTransform(request, transform) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "tools/run" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function toolsRunRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "tools/run/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools run request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function toolsRunRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "tools/run/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools run request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function toolsRunResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "tools/run/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools run response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function toolsRunResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "tools/run/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
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
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "update" }), zod.z.union([CliErrorSchema, CliCommandUpdateResponseItemSchema]));
}
function updateExecuteTransform(request, transform) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "update" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function updateRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "update/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("update request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function updateRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "update/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("update request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function updateResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "update/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("update response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function updateResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "update/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("update response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandViewerConfigAddressGetResponseSchema = zod.z.object({
  address: zod.z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.viewer.config.address.get.Response" });

// src/viewer/command/viewer/config/address/get.ts
async function viewerConfigAddressGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/config/address/get" }), zod.z.union([CliErrorSchema, CliCommandViewerConfigAddressGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config address get: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigAddressGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/config/address/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config address get: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigAddressGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/config/address/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config address get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigAddressGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/config/address/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config address get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigAddressGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/config/address/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config address get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigAddressGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/config/address/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config address get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigAddressSetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "viewer/config/address/set" }), zod.z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config address set: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigAddressSetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "viewer/config/address/set" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config address set: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigAddressSetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/config/address/set/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config address set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigAddressSetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/config/address/set/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config address set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigAddressSetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/config/address/set/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config address set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigAddressSetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/config/address/set/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config address set response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandViewerConfigGetResponseSchema = zod.z.object({
  address: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  secret: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  signature: zod.z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.viewer.config.get.Response" });

// src/viewer/command/viewer/config/get.ts
async function viewerConfigGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/config/get" }), zod.z.union([CliErrorSchema, CliCommandViewerConfigGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config get: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/config/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config get: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/config/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/config/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/config/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/config/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandViewerConfigSecretGetResponseSchema = zod.z.object({
  secret: zod.z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.viewer.config.secret.get.Response" });

// src/viewer/command/viewer/config/secret/get.ts
async function viewerConfigSecretGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/config/secret/get" }), zod.z.union([CliErrorSchema, CliCommandViewerConfigSecretGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config secret get: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigSecretGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/config/secret/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config secret get: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigSecretGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/config/secret/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config secret get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigSecretGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/config/secret/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config secret get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigSecretGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/config/secret/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config secret get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigSecretGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/config/secret/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config secret get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigSecretSetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "viewer/config/secret/set" }), zod.z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config secret set: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigSecretSetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "viewer/config/secret/set" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config secret set: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigSecretSetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/config/secret/set/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config secret set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigSecretSetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/config/secret/set/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config secret set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigSecretSetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/config/secret/set/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config secret set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigSecretSetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/config/secret/set/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config secret set response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandViewerConfigSignatureGetResponseSchema = zod.z.object({
  signature: zod.z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.viewer.config.signature.get.Response" });

// src/viewer/command/viewer/config/signature/get.ts
async function viewerConfigSignatureGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/config/signature/get" }), zod.z.union([CliErrorSchema, CliCommandViewerConfigSignatureGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config signature get: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigSignatureGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/config/signature/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config signature get: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigSignatureGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/config/signature/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config signature get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigSignatureGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/config/signature/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config signature get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigSignatureGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/config/signature/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config signature get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigSignatureGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/config/signature/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config signature get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigSignatureSetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "viewer/config/signature/set" }), zod.z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config signature set: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigSignatureSetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "viewer/config/signature/set" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config signature set: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigSignatureSetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/config/signature/set/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config signature set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigSignatureSetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/config/signature/set/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config signature set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigSignatureSetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/config/signature/set/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config signature set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigSignatureSetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/config/signature/set/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config signature set response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandViewerGenerateSecretSignaturePairResponseSchema = zod.z.object({
  secret: zod.z.string(),
  signature: zod.z.string()
}).meta({ title: "cli.command.viewer.generate_secret_signature_pair.Response" });

// src/viewer/command/viewer/generate_secret_signature_pair.ts
async function viewerGenerateSecretSignaturePairExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/generate_secret_signature_pair" }), zod.z.union([CliErrorSchema, CliCommandViewerGenerateSecretSignaturePairResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer generate_secret_signature_pair: cli produced no output before the end marker");
  }
  return first;
}
async function viewerGenerateSecretSignaturePairExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/generate_secret_signature_pair" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer generate_secret_signature_pair: cli produced no output before the end marker");
  }
  return first;
}
async function viewerGenerateSecretSignaturePairRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/generate_secret_signature_pair/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer generate_secret_signature_pair request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerGenerateSecretSignaturePairRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/generate_secret_signature_pair/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer generate_secret_signature_pair request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerGenerateSecretSignaturePairResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/generate_secret_signature_pair/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer generate_secret_signature_pair response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerGenerateSecretSignaturePairResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/generate_secret_signature_pair/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
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
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/kill" }), zod.z.union([CliErrorSchema, CliCommandViewerKillResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer kill: cli produced no output before the end marker");
  }
  return first;
}
async function viewerKillExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/kill" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer kill: cli produced no output before the end marker");
  }
  return first;
}
async function viewerKillRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/kill/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer kill request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerKillRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/kill/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer kill request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerKillResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/kill/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer kill response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerKillResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/kill/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
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
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/send" }), zod.z.union([CliErrorSchema, CliCommandViewerSendResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer send: cli produced no output before the end marker");
  }
  return first;
}
async function viewerSendExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/send" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer send: cli produced no output before the end marker");
  }
  return first;
}
async function viewerSendRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/send/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer send request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerSendRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/send/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer send request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerSendResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/send/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer send response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerSendResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/send/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
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
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/spawn" }), zod.z.union([CliErrorSchema, CliCommandViewerSpawnResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer spawn: cli produced no output before the end marker");
  }
  return first;
}
async function viewerSpawnExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/spawn" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer spawn: cli produced no output before the end marker");
  }
  return first;
}
async function viewerSpawnRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/spawn/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer spawn request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerSpawnRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/spawn/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer spawn request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerSpawnResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/spawn/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer spawn response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerSpawnResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/spawn/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
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
exports.agentsEnqueueExecute = agentsEnqueueExecute;
exports.agentsEnqueueExecuteTransform = agentsEnqueueExecuteTransform;
exports.agentsEnqueueRequestSchemaExecute = agentsEnqueueRequestSchemaExecute;
exports.agentsEnqueueRequestSchemaExecuteTransform = agentsEnqueueRequestSchemaExecuteTransform;
exports.agentsEnqueueResponseSchemaExecute = agentsEnqueueResponseSchemaExecute;
exports.agentsEnqueueResponseSchemaExecuteTransform = agentsEnqueueResponseSchemaExecuteTransform;
exports.agentsGetExecute = agentsGetExecute;
exports.agentsGetExecuteTransform = agentsGetExecuteTransform;
exports.agentsGetRequestSchemaExecute = agentsGetRequestSchemaExecute;
exports.agentsGetRequestSchemaExecuteTransform = agentsGetRequestSchemaExecuteTransform;
exports.agentsGetResponseSchemaExecute = agentsGetResponseSchemaExecute;
exports.agentsGetResponseSchemaExecuteTransform = agentsGetResponseSchemaExecuteTransform;
exports.agentsInstancesGetExecute = agentsInstancesGetExecute;
exports.agentsInstancesGetExecuteTransform = agentsInstancesGetExecuteTransform;
exports.agentsInstancesGetRequestSchemaExecute = agentsInstancesGetRequestSchemaExecute;
exports.agentsInstancesGetRequestSchemaExecuteTransform = agentsInstancesGetRequestSchemaExecuteTransform;
exports.agentsInstancesGetResponseSchemaExecute = agentsInstancesGetResponseSchemaExecute;
exports.agentsInstancesGetResponseSchemaExecuteTransform = agentsInstancesGetResponseSchemaExecuteTransform;
exports.agentsInstancesListExecute = agentsInstancesListExecute;
exports.agentsInstancesListExecuteTransform = agentsInstancesListExecuteTransform;
exports.agentsInstancesListRequestSchemaExecute = agentsInstancesListRequestSchemaExecute;
exports.agentsInstancesListRequestSchemaExecuteTransform = agentsInstancesListRequestSchemaExecuteTransform;
exports.agentsInstancesListResponseSchemaExecute = agentsInstancesListResponseSchemaExecute;
exports.agentsInstancesListResponseSchemaExecuteTransform = agentsInstancesListResponseSchemaExecuteTransform;
exports.agentsListExecute = agentsListExecute;
exports.agentsListExecuteTransform = agentsListExecuteTransform;
exports.agentsListRequestSchemaExecute = agentsListRequestSchemaExecute;
exports.agentsListRequestSchemaExecuteTransform = agentsListRequestSchemaExecuteTransform;
exports.agentsListResponseSchemaExecute = agentsListResponseSchemaExecute;
exports.agentsListResponseSchemaExecuteTransform = agentsListResponseSchemaExecuteTransform;
exports.agentsLogsReadAllExecute = agentsLogsReadAllExecute;
exports.agentsLogsReadAllExecuteTransform = agentsLogsReadAllExecuteTransform;
exports.agentsLogsReadAllRequestSchemaExecute = agentsLogsReadAllRequestSchemaExecute;
exports.agentsLogsReadAllRequestSchemaExecuteTransform = agentsLogsReadAllRequestSchemaExecuteTransform;
exports.agentsLogsReadAllResponseSchemaExecute = agentsLogsReadAllResponseSchemaExecute;
exports.agentsLogsReadAllResponseSchemaExecuteTransform = agentsLogsReadAllResponseSchemaExecuteTransform;
exports.agentsLogsReadIdExecute = agentsLogsReadIdExecute;
exports.agentsLogsReadIdExecuteTransform = agentsLogsReadIdExecuteTransform;
exports.agentsLogsReadIdRequestSchemaExecute = agentsLogsReadIdRequestSchemaExecute;
exports.agentsLogsReadIdRequestSchemaExecuteTransform = agentsLogsReadIdRequestSchemaExecuteTransform;
exports.agentsLogsReadIdResponseSchemaExecute = agentsLogsReadIdResponseSchemaExecute;
exports.agentsLogsReadIdResponseSchemaExecuteTransform = agentsLogsReadIdResponseSchemaExecuteTransform;
exports.agentsLogsReadPendingExecute = agentsLogsReadPendingExecute;
exports.agentsLogsReadPendingExecuteTransform = agentsLogsReadPendingExecuteTransform;
exports.agentsLogsReadPendingRequestSchemaExecute = agentsLogsReadPendingRequestSchemaExecute;
exports.agentsLogsReadPendingRequestSchemaExecuteTransform = agentsLogsReadPendingRequestSchemaExecuteTransform;
exports.agentsLogsReadPendingResponseSchemaExecute = agentsLogsReadPendingResponseSchemaExecute;
exports.agentsLogsReadPendingResponseSchemaExecuteTransform = agentsLogsReadPendingResponseSchemaExecuteTransform;
exports.agentsLogsReadSubscribeExecute = agentsLogsReadSubscribeExecute;
exports.agentsLogsReadSubscribeExecuteTransform = agentsLogsReadSubscribeExecuteTransform;
exports.agentsLogsReadSubscribeRequestSchemaExecute = agentsLogsReadSubscribeRequestSchemaExecute;
exports.agentsLogsReadSubscribeRequestSchemaExecuteTransform = agentsLogsReadSubscribeRequestSchemaExecuteTransform;
exports.agentsLogsReadSubscribeResponseSchemaExecute = agentsLogsReadSubscribeResponseSchemaExecute;
exports.agentsLogsReadSubscribeResponseSchemaExecuteTransform = agentsLogsReadSubscribeResponseSchemaExecuteTransform;
exports.agentsMessageExecute = agentsMessageExecute;
exports.agentsMessageExecuteTransform = agentsMessageExecuteTransform;
exports.agentsMessageRequestSchemaExecute = agentsMessageRequestSchemaExecute;
exports.agentsMessageRequestSchemaExecuteTransform = agentsMessageRequestSchemaExecuteTransform;
exports.agentsMessageResponseSchemaExecute = agentsMessageResponseSchemaExecute;
exports.agentsMessageResponseSchemaExecuteTransform = agentsMessageResponseSchemaExecuteTransform;
exports.agentsPublishExecute = agentsPublishExecute;
exports.agentsPublishExecuteTransform = agentsPublishExecuteTransform;
exports.agentsPublishRequestSchemaExecute = agentsPublishRequestSchemaExecute;
exports.agentsPublishRequestSchemaExecuteTransform = agentsPublishRequestSchemaExecuteTransform;
exports.agentsPublishResponseSchemaExecute = agentsPublishResponseSchemaExecute;
exports.agentsPublishResponseSchemaExecuteTransform = agentsPublishResponseSchemaExecuteTransform;
exports.agentsQueueDeleteExecute = agentsQueueDeleteExecute;
exports.agentsQueueDeleteExecuteTransform = agentsQueueDeleteExecuteTransform;
exports.agentsQueueDeleteRequestSchemaExecute = agentsQueueDeleteRequestSchemaExecute;
exports.agentsQueueDeleteRequestSchemaExecuteTransform = agentsQueueDeleteRequestSchemaExecuteTransform;
exports.agentsQueueDeleteResponseSchemaExecute = agentsQueueDeleteResponseSchemaExecute;
exports.agentsQueueDeleteResponseSchemaExecuteTransform = agentsQueueDeleteResponseSchemaExecuteTransform;
exports.agentsQueueDeliverExecute = agentsQueueDeliverExecute;
exports.agentsQueueDeliverExecuteTransform = agentsQueueDeliverExecuteTransform;
exports.agentsQueueDeliverRequestSchemaExecute = agentsQueueDeliverRequestSchemaExecute;
exports.agentsQueueDeliverRequestSchemaExecuteTransform = agentsQueueDeliverRequestSchemaExecuteTransform;
exports.agentsQueueDeliverResponseSchemaExecute = agentsQueueDeliverResponseSchemaExecute;
exports.agentsQueueDeliverResponseSchemaExecuteTransform = agentsQueueDeliverResponseSchemaExecuteTransform;
exports.agentsQueueReadIdExecute = agentsQueueReadIdExecute;
exports.agentsQueueReadIdExecuteTransform = agentsQueueReadIdExecuteTransform;
exports.agentsQueueReadIdRequestSchemaExecute = agentsQueueReadIdRequestSchemaExecute;
exports.agentsQueueReadIdRequestSchemaExecuteTransform = agentsQueueReadIdRequestSchemaExecuteTransform;
exports.agentsQueueReadIdResponseSchemaExecute = agentsQueueReadIdResponseSchemaExecute;
exports.agentsQueueReadIdResponseSchemaExecuteTransform = agentsQueueReadIdResponseSchemaExecuteTransform;
exports.agentsQueueReadPendingExecute = agentsQueueReadPendingExecute;
exports.agentsQueueReadPendingExecuteTransform = agentsQueueReadPendingExecuteTransform;
exports.agentsQueueReadPendingRequestSchemaExecute = agentsQueueReadPendingRequestSchemaExecute;
exports.agentsQueueReadPendingRequestSchemaExecuteTransform = agentsQueueReadPendingRequestSchemaExecuteTransform;
exports.agentsQueueReadPendingResponseSchemaExecute = agentsQueueReadPendingResponseSchemaExecute;
exports.agentsQueueReadPendingResponseSchemaExecuteTransform = agentsQueueReadPendingResponseSchemaExecuteTransform;
exports.agentsSpawnExecute = agentsSpawnExecute;
exports.agentsSpawnExecuteStreaming = agentsSpawnExecuteStreaming;
exports.agentsSpawnExecuteStreamingTransform = agentsSpawnExecuteStreamingTransform;
exports.agentsSpawnExecuteTransform = agentsSpawnExecuteTransform;
exports.agentsSpawnRequestSchemaExecute = agentsSpawnRequestSchemaExecute;
exports.agentsSpawnRequestSchemaExecuteTransform = agentsSpawnRequestSchemaExecuteTransform;
exports.agentsSpawnResponseSchemaExecute = agentsSpawnResponseSchemaExecute;
exports.agentsSpawnResponseSchemaExecuteTransform = agentsSpawnResponseSchemaExecuteTransform;
exports.agentsTagsApplyExecute = agentsTagsApplyExecute;
exports.agentsTagsApplyExecuteTransform = agentsTagsApplyExecuteTransform;
exports.agentsTagsApplyRequestSchemaExecute = agentsTagsApplyRequestSchemaExecute;
exports.agentsTagsApplyRequestSchemaExecuteTransform = agentsTagsApplyRequestSchemaExecuteTransform;
exports.agentsTagsApplyResponseSchemaExecute = agentsTagsApplyResponseSchemaExecute;
exports.agentsTagsApplyResponseSchemaExecuteTransform = agentsTagsApplyResponseSchemaExecuteTransform;
exports.agentsTagsLookupRequestSchemaExecute = agentsTagsLookupRequestSchemaExecute;
exports.agentsTagsLookupRequestSchemaExecuteTransform = agentsTagsLookupRequestSchemaExecuteTransform;
exports.agentsTagsLookupResponseSchemaExecute = agentsTagsLookupResponseSchemaExecute;
exports.agentsTagsLookupResponseSchemaExecuteTransform = agentsTagsLookupResponseSchemaExecuteTransform;
exports.agentsWaitExecute = agentsWaitExecute;
exports.agentsWaitExecuteTransform = agentsWaitExecuteTransform;
exports.agentsWaitRequestSchemaExecute = agentsWaitRequestSchemaExecute;
exports.agentsWaitRequestSchemaExecuteTransform = agentsWaitRequestSchemaExecuteTransform;
exports.agentsWaitResponseSchemaExecute = agentsWaitResponseSchemaExecute;
exports.agentsWaitResponseSchemaExecuteTransform = agentsWaitResponseSchemaExecuteTransform;
exports.apiConfigAddressGetExecute = apiConfigAddressGetExecute;
exports.apiConfigAddressGetExecuteTransform = apiConfigAddressGetExecuteTransform;
exports.apiConfigAddressGetRequestSchemaExecute = apiConfigAddressGetRequestSchemaExecute;
exports.apiConfigAddressGetRequestSchemaExecuteTransform = apiConfigAddressGetRequestSchemaExecuteTransform;
exports.apiConfigAddressGetResponseSchemaExecute = apiConfigAddressGetResponseSchemaExecute;
exports.apiConfigAddressGetResponseSchemaExecuteTransform = apiConfigAddressGetResponseSchemaExecuteTransform;
exports.apiConfigAddressSetExecute = apiConfigAddressSetExecute;
exports.apiConfigAddressSetExecuteTransform = apiConfigAddressSetExecuteTransform;
exports.apiConfigAddressSetRequestSchemaExecute = apiConfigAddressSetRequestSchemaExecute;
exports.apiConfigAddressSetRequestSchemaExecuteTransform = apiConfigAddressSetRequestSchemaExecuteTransform;
exports.apiConfigAddressSetResponseSchemaExecute = apiConfigAddressSetResponseSchemaExecute;
exports.apiConfigAddressSetResponseSchemaExecuteTransform = apiConfigAddressSetResponseSchemaExecuteTransform;
exports.apiConfigCommitAuthorEmailGetExecute = apiConfigCommitAuthorEmailGetExecute;
exports.apiConfigCommitAuthorEmailGetExecuteTransform = apiConfigCommitAuthorEmailGetExecuteTransform;
exports.apiConfigCommitAuthorEmailGetRequestSchemaExecute = apiConfigCommitAuthorEmailGetRequestSchemaExecute;
exports.apiConfigCommitAuthorEmailGetRequestSchemaExecuteTransform = apiConfigCommitAuthorEmailGetRequestSchemaExecuteTransform;
exports.apiConfigCommitAuthorEmailGetResponseSchemaExecute = apiConfigCommitAuthorEmailGetResponseSchemaExecute;
exports.apiConfigCommitAuthorEmailGetResponseSchemaExecuteTransform = apiConfigCommitAuthorEmailGetResponseSchemaExecuteTransform;
exports.apiConfigCommitAuthorEmailSetExecute = apiConfigCommitAuthorEmailSetExecute;
exports.apiConfigCommitAuthorEmailSetExecuteTransform = apiConfigCommitAuthorEmailSetExecuteTransform;
exports.apiConfigCommitAuthorEmailSetRequestSchemaExecute = apiConfigCommitAuthorEmailSetRequestSchemaExecute;
exports.apiConfigCommitAuthorEmailSetRequestSchemaExecuteTransform = apiConfigCommitAuthorEmailSetRequestSchemaExecuteTransform;
exports.apiConfigCommitAuthorEmailSetResponseSchemaExecute = apiConfigCommitAuthorEmailSetResponseSchemaExecute;
exports.apiConfigCommitAuthorEmailSetResponseSchemaExecuteTransform = apiConfigCommitAuthorEmailSetResponseSchemaExecuteTransform;
exports.apiConfigCommitAuthorNameGetExecute = apiConfigCommitAuthorNameGetExecute;
exports.apiConfigCommitAuthorNameGetExecuteTransform = apiConfigCommitAuthorNameGetExecuteTransform;
exports.apiConfigCommitAuthorNameGetRequestSchemaExecute = apiConfigCommitAuthorNameGetRequestSchemaExecute;
exports.apiConfigCommitAuthorNameGetRequestSchemaExecuteTransform = apiConfigCommitAuthorNameGetRequestSchemaExecuteTransform;
exports.apiConfigCommitAuthorNameGetResponseSchemaExecute = apiConfigCommitAuthorNameGetResponseSchemaExecute;
exports.apiConfigCommitAuthorNameGetResponseSchemaExecuteTransform = apiConfigCommitAuthorNameGetResponseSchemaExecuteTransform;
exports.apiConfigCommitAuthorNameSetExecute = apiConfigCommitAuthorNameSetExecute;
exports.apiConfigCommitAuthorNameSetExecuteTransform = apiConfigCommitAuthorNameSetExecuteTransform;
exports.apiConfigCommitAuthorNameSetRequestSchemaExecute = apiConfigCommitAuthorNameSetRequestSchemaExecute;
exports.apiConfigCommitAuthorNameSetRequestSchemaExecuteTransform = apiConfigCommitAuthorNameSetRequestSchemaExecuteTransform;
exports.apiConfigCommitAuthorNameSetResponseSchemaExecute = apiConfigCommitAuthorNameSetResponseSchemaExecute;
exports.apiConfigCommitAuthorNameSetResponseSchemaExecuteTransform = apiConfigCommitAuthorNameSetResponseSchemaExecuteTransform;
exports.apiConfigGetExecute = apiConfigGetExecute;
exports.apiConfigGetExecuteTransform = apiConfigGetExecuteTransform;
exports.apiConfigGetRequestSchemaExecute = apiConfigGetRequestSchemaExecute;
exports.apiConfigGetRequestSchemaExecuteTransform = apiConfigGetRequestSchemaExecuteTransform;
exports.apiConfigGetResponseSchemaExecute = apiConfigGetResponseSchemaExecute;
exports.apiConfigGetResponseSchemaExecuteTransform = apiConfigGetResponseSchemaExecuteTransform;
exports.apiConfigGithubAuthorizationGetExecute = apiConfigGithubAuthorizationGetExecute;
exports.apiConfigGithubAuthorizationGetExecuteTransform = apiConfigGithubAuthorizationGetExecuteTransform;
exports.apiConfigGithubAuthorizationGetRequestSchemaExecute = apiConfigGithubAuthorizationGetRequestSchemaExecute;
exports.apiConfigGithubAuthorizationGetRequestSchemaExecuteTransform = apiConfigGithubAuthorizationGetRequestSchemaExecuteTransform;
exports.apiConfigGithubAuthorizationGetResponseSchemaExecute = apiConfigGithubAuthorizationGetResponseSchemaExecute;
exports.apiConfigGithubAuthorizationGetResponseSchemaExecuteTransform = apiConfigGithubAuthorizationGetResponseSchemaExecuteTransform;
exports.apiConfigGithubAuthorizationSetExecute = apiConfigGithubAuthorizationSetExecute;
exports.apiConfigGithubAuthorizationSetExecuteTransform = apiConfigGithubAuthorizationSetExecuteTransform;
exports.apiConfigGithubAuthorizationSetRequestSchemaExecute = apiConfigGithubAuthorizationSetRequestSchemaExecute;
exports.apiConfigGithubAuthorizationSetRequestSchemaExecuteTransform = apiConfigGithubAuthorizationSetRequestSchemaExecuteTransform;
exports.apiConfigGithubAuthorizationSetResponseSchemaExecute = apiConfigGithubAuthorizationSetResponseSchemaExecute;
exports.apiConfigGithubAuthorizationSetResponseSchemaExecuteTransform = apiConfigGithubAuthorizationSetResponseSchemaExecuteTransform;
exports.apiConfigHttpRefererGetExecute = apiConfigHttpRefererGetExecute;
exports.apiConfigHttpRefererGetExecuteTransform = apiConfigHttpRefererGetExecuteTransform;
exports.apiConfigHttpRefererGetRequestSchemaExecute = apiConfigHttpRefererGetRequestSchemaExecute;
exports.apiConfigHttpRefererGetRequestSchemaExecuteTransform = apiConfigHttpRefererGetRequestSchemaExecuteTransform;
exports.apiConfigHttpRefererGetResponseSchemaExecute = apiConfigHttpRefererGetResponseSchemaExecute;
exports.apiConfigHttpRefererGetResponseSchemaExecuteTransform = apiConfigHttpRefererGetResponseSchemaExecuteTransform;
exports.apiConfigHttpRefererSetExecute = apiConfigHttpRefererSetExecute;
exports.apiConfigHttpRefererSetExecuteTransform = apiConfigHttpRefererSetExecuteTransform;
exports.apiConfigHttpRefererSetRequestSchemaExecute = apiConfigHttpRefererSetRequestSchemaExecute;
exports.apiConfigHttpRefererSetRequestSchemaExecuteTransform = apiConfigHttpRefererSetRequestSchemaExecuteTransform;
exports.apiConfigHttpRefererSetResponseSchemaExecute = apiConfigHttpRefererSetResponseSchemaExecute;
exports.apiConfigHttpRefererSetResponseSchemaExecuteTransform = apiConfigHttpRefererSetResponseSchemaExecuteTransform;
exports.apiConfigMcpAuthorizationAddExecute = apiConfigMcpAuthorizationAddExecute;
exports.apiConfigMcpAuthorizationAddExecuteTransform = apiConfigMcpAuthorizationAddExecuteTransform;
exports.apiConfigMcpAuthorizationAddRequestSchemaExecute = apiConfigMcpAuthorizationAddRequestSchemaExecute;
exports.apiConfigMcpAuthorizationAddRequestSchemaExecuteTransform = apiConfigMcpAuthorizationAddRequestSchemaExecuteTransform;
exports.apiConfigMcpAuthorizationAddResponseSchemaExecute = apiConfigMcpAuthorizationAddResponseSchemaExecute;
exports.apiConfigMcpAuthorizationAddResponseSchemaExecuteTransform = apiConfigMcpAuthorizationAddResponseSchemaExecuteTransform;
exports.apiConfigMcpAuthorizationDelExecute = apiConfigMcpAuthorizationDelExecute;
exports.apiConfigMcpAuthorizationDelExecuteTransform = apiConfigMcpAuthorizationDelExecuteTransform;
exports.apiConfigMcpAuthorizationDelRequestSchemaExecute = apiConfigMcpAuthorizationDelRequestSchemaExecute;
exports.apiConfigMcpAuthorizationDelRequestSchemaExecuteTransform = apiConfigMcpAuthorizationDelRequestSchemaExecuteTransform;
exports.apiConfigMcpAuthorizationDelResponseSchemaExecute = apiConfigMcpAuthorizationDelResponseSchemaExecute;
exports.apiConfigMcpAuthorizationDelResponseSchemaExecuteTransform = apiConfigMcpAuthorizationDelResponseSchemaExecuteTransform;
exports.apiConfigMcpAuthorizationGetExecute = apiConfigMcpAuthorizationGetExecute;
exports.apiConfigMcpAuthorizationGetExecuteTransform = apiConfigMcpAuthorizationGetExecuteTransform;
exports.apiConfigMcpAuthorizationGetRequestSchemaExecute = apiConfigMcpAuthorizationGetRequestSchemaExecute;
exports.apiConfigMcpAuthorizationGetRequestSchemaExecuteTransform = apiConfigMcpAuthorizationGetRequestSchemaExecuteTransform;
exports.apiConfigMcpAuthorizationGetResponseSchemaExecute = apiConfigMcpAuthorizationGetResponseSchemaExecute;
exports.apiConfigMcpAuthorizationGetResponseSchemaExecuteTransform = apiConfigMcpAuthorizationGetResponseSchemaExecuteTransform;
exports.apiConfigObjectiveaiAuthorizationGetExecute = apiConfigObjectiveaiAuthorizationGetExecute;
exports.apiConfigObjectiveaiAuthorizationGetExecuteTransform = apiConfigObjectiveaiAuthorizationGetExecuteTransform;
exports.apiConfigObjectiveaiAuthorizationGetRequestSchemaExecute = apiConfigObjectiveaiAuthorizationGetRequestSchemaExecute;
exports.apiConfigObjectiveaiAuthorizationGetRequestSchemaExecuteTransform = apiConfigObjectiveaiAuthorizationGetRequestSchemaExecuteTransform;
exports.apiConfigObjectiveaiAuthorizationGetResponseSchemaExecute = apiConfigObjectiveaiAuthorizationGetResponseSchemaExecute;
exports.apiConfigObjectiveaiAuthorizationGetResponseSchemaExecuteTransform = apiConfigObjectiveaiAuthorizationGetResponseSchemaExecuteTransform;
exports.apiConfigObjectiveaiAuthorizationSetExecute = apiConfigObjectiveaiAuthorizationSetExecute;
exports.apiConfigObjectiveaiAuthorizationSetExecuteTransform = apiConfigObjectiveaiAuthorizationSetExecuteTransform;
exports.apiConfigObjectiveaiAuthorizationSetRequestSchemaExecute = apiConfigObjectiveaiAuthorizationSetRequestSchemaExecute;
exports.apiConfigObjectiveaiAuthorizationSetRequestSchemaExecuteTransform = apiConfigObjectiveaiAuthorizationSetRequestSchemaExecuteTransform;
exports.apiConfigObjectiveaiAuthorizationSetResponseSchemaExecute = apiConfigObjectiveaiAuthorizationSetResponseSchemaExecute;
exports.apiConfigObjectiveaiAuthorizationSetResponseSchemaExecuteTransform = apiConfigObjectiveaiAuthorizationSetResponseSchemaExecuteTransform;
exports.apiConfigOpenrouterAuthorizationGetExecute = apiConfigOpenrouterAuthorizationGetExecute;
exports.apiConfigOpenrouterAuthorizationGetExecuteTransform = apiConfigOpenrouterAuthorizationGetExecuteTransform;
exports.apiConfigOpenrouterAuthorizationGetRequestSchemaExecute = apiConfigOpenrouterAuthorizationGetRequestSchemaExecute;
exports.apiConfigOpenrouterAuthorizationGetRequestSchemaExecuteTransform = apiConfigOpenrouterAuthorizationGetRequestSchemaExecuteTransform;
exports.apiConfigOpenrouterAuthorizationGetResponseSchemaExecute = apiConfigOpenrouterAuthorizationGetResponseSchemaExecute;
exports.apiConfigOpenrouterAuthorizationGetResponseSchemaExecuteTransform = apiConfigOpenrouterAuthorizationGetResponseSchemaExecuteTransform;
exports.apiConfigOpenrouterAuthorizationSetExecute = apiConfigOpenrouterAuthorizationSetExecute;
exports.apiConfigOpenrouterAuthorizationSetExecuteTransform = apiConfigOpenrouterAuthorizationSetExecuteTransform;
exports.apiConfigOpenrouterAuthorizationSetRequestSchemaExecute = apiConfigOpenrouterAuthorizationSetRequestSchemaExecute;
exports.apiConfigOpenrouterAuthorizationSetRequestSchemaExecuteTransform = apiConfigOpenrouterAuthorizationSetRequestSchemaExecuteTransform;
exports.apiConfigOpenrouterAuthorizationSetResponseSchemaExecute = apiConfigOpenrouterAuthorizationSetResponseSchemaExecute;
exports.apiConfigOpenrouterAuthorizationSetResponseSchemaExecuteTransform = apiConfigOpenrouterAuthorizationSetResponseSchemaExecuteTransform;
exports.apiConfigUserAgentGetExecute = apiConfigUserAgentGetExecute;
exports.apiConfigUserAgentGetExecuteTransform = apiConfigUserAgentGetExecuteTransform;
exports.apiConfigUserAgentGetRequestSchemaExecute = apiConfigUserAgentGetRequestSchemaExecute;
exports.apiConfigUserAgentGetRequestSchemaExecuteTransform = apiConfigUserAgentGetRequestSchemaExecuteTransform;
exports.apiConfigUserAgentGetResponseSchemaExecute = apiConfigUserAgentGetResponseSchemaExecute;
exports.apiConfigUserAgentGetResponseSchemaExecuteTransform = apiConfigUserAgentGetResponseSchemaExecuteTransform;
exports.apiConfigUserAgentSetExecute = apiConfigUserAgentSetExecute;
exports.apiConfigUserAgentSetExecuteTransform = apiConfigUserAgentSetExecuteTransform;
exports.apiConfigUserAgentSetRequestSchemaExecute = apiConfigUserAgentSetRequestSchemaExecute;
exports.apiConfigUserAgentSetRequestSchemaExecuteTransform = apiConfigUserAgentSetRequestSchemaExecuteTransform;
exports.apiConfigUserAgentSetResponseSchemaExecute = apiConfigUserAgentSetResponseSchemaExecute;
exports.apiConfigUserAgentSetResponseSchemaExecuteTransform = apiConfigUserAgentSetResponseSchemaExecuteTransform;
exports.apiConfigXTitleGetExecute = apiConfigXTitleGetExecute;
exports.apiConfigXTitleGetExecuteTransform = apiConfigXTitleGetExecuteTransform;
exports.apiConfigXTitleGetRequestSchemaExecute = apiConfigXTitleGetRequestSchemaExecute;
exports.apiConfigXTitleGetRequestSchemaExecuteTransform = apiConfigXTitleGetRequestSchemaExecuteTransform;
exports.apiConfigXTitleGetResponseSchemaExecute = apiConfigXTitleGetResponseSchemaExecute;
exports.apiConfigXTitleGetResponseSchemaExecuteTransform = apiConfigXTitleGetResponseSchemaExecuteTransform;
exports.apiConfigXTitleSetExecute = apiConfigXTitleSetExecute;
exports.apiConfigXTitleSetExecuteTransform = apiConfigXTitleSetExecuteTransform;
exports.apiConfigXTitleSetRequestSchemaExecute = apiConfigXTitleSetRequestSchemaExecute;
exports.apiConfigXTitleSetRequestSchemaExecuteTransform = apiConfigXTitleSetRequestSchemaExecuteTransform;
exports.apiConfigXTitleSetResponseSchemaExecute = apiConfigXTitleSetResponseSchemaExecute;
exports.apiConfigXTitleSetResponseSchemaExecuteTransform = apiConfigXTitleSetResponseSchemaExecuteTransform;
exports.apiKillExecute = apiKillExecute;
exports.apiKillExecuteTransform = apiKillExecuteTransform;
exports.apiKillRequestSchemaExecute = apiKillRequestSchemaExecute;
exports.apiKillRequestSchemaExecuteTransform = apiKillRequestSchemaExecuteTransform;
exports.apiKillResponseSchemaExecute = apiKillResponseSchemaExecute;
exports.apiKillResponseSchemaExecuteTransform = apiKillResponseSchemaExecuteTransform;
exports.apiSpawnExecute = apiSpawnExecute;
exports.apiSpawnExecuteTransform = apiSpawnExecuteTransform;
exports.apiSpawnRequestSchemaExecute = apiSpawnRequestSchemaExecute;
exports.apiSpawnRequestSchemaExecuteTransform = apiSpawnRequestSchemaExecuteTransform;
exports.apiSpawnResponseSchemaExecute = apiSpawnResponseSchemaExecute;
exports.apiSpawnResponseSchemaExecuteTransform = apiSpawnResponseSchemaExecuteTransform;
exports.dbConfigAddressGetExecute = dbConfigAddressGetExecute;
exports.dbConfigAddressGetExecuteTransform = dbConfigAddressGetExecuteTransform;
exports.dbConfigAddressGetRequestSchemaExecute = dbConfigAddressGetRequestSchemaExecute;
exports.dbConfigAddressGetRequestSchemaExecuteTransform = dbConfigAddressGetRequestSchemaExecuteTransform;
exports.dbConfigAddressGetResponseSchemaExecute = dbConfigAddressGetResponseSchemaExecute;
exports.dbConfigAddressGetResponseSchemaExecuteTransform = dbConfigAddressGetResponseSchemaExecuteTransform;
exports.dbConfigAddressSetExecute = dbConfigAddressSetExecute;
exports.dbConfigAddressSetExecuteTransform = dbConfigAddressSetExecuteTransform;
exports.dbConfigAddressSetRequestSchemaExecute = dbConfigAddressSetRequestSchemaExecute;
exports.dbConfigAddressSetRequestSchemaExecuteTransform = dbConfigAddressSetRequestSchemaExecuteTransform;
exports.dbConfigAddressSetResponseSchemaExecute = dbConfigAddressSetResponseSchemaExecute;
exports.dbConfigAddressSetResponseSchemaExecuteTransform = dbConfigAddressSetResponseSchemaExecuteTransform;
exports.dbConfigDatabaseGetExecute = dbConfigDatabaseGetExecute;
exports.dbConfigDatabaseGetExecuteTransform = dbConfigDatabaseGetExecuteTransform;
exports.dbConfigDatabaseGetRequestSchemaExecute = dbConfigDatabaseGetRequestSchemaExecute;
exports.dbConfigDatabaseGetRequestSchemaExecuteTransform = dbConfigDatabaseGetRequestSchemaExecuteTransform;
exports.dbConfigDatabaseGetResponseSchemaExecute = dbConfigDatabaseGetResponseSchemaExecute;
exports.dbConfigDatabaseGetResponseSchemaExecuteTransform = dbConfigDatabaseGetResponseSchemaExecuteTransform;
exports.dbConfigDatabaseSetExecute = dbConfigDatabaseSetExecute;
exports.dbConfigDatabaseSetExecuteTransform = dbConfigDatabaseSetExecuteTransform;
exports.dbConfigDatabaseSetRequestSchemaExecute = dbConfigDatabaseSetRequestSchemaExecute;
exports.dbConfigDatabaseSetRequestSchemaExecuteTransform = dbConfigDatabaseSetRequestSchemaExecuteTransform;
exports.dbConfigDatabaseSetResponseSchemaExecute = dbConfigDatabaseSetResponseSchemaExecute;
exports.dbConfigDatabaseSetResponseSchemaExecuteTransform = dbConfigDatabaseSetResponseSchemaExecuteTransform;
exports.dbConfigGetExecute = dbConfigGetExecute;
exports.dbConfigGetExecuteTransform = dbConfigGetExecuteTransform;
exports.dbConfigGetRequestSchemaExecute = dbConfigGetRequestSchemaExecute;
exports.dbConfigGetRequestSchemaExecuteTransform = dbConfigGetRequestSchemaExecuteTransform;
exports.dbConfigGetResponseSchemaExecute = dbConfigGetResponseSchemaExecute;
exports.dbConfigGetResponseSchemaExecuteTransform = dbConfigGetResponseSchemaExecuteTransform;
exports.dbConfigPasswordGetExecute = dbConfigPasswordGetExecute;
exports.dbConfigPasswordGetExecuteTransform = dbConfigPasswordGetExecuteTransform;
exports.dbConfigPasswordGetRequestSchemaExecute = dbConfigPasswordGetRequestSchemaExecute;
exports.dbConfigPasswordGetRequestSchemaExecuteTransform = dbConfigPasswordGetRequestSchemaExecuteTransform;
exports.dbConfigPasswordGetResponseSchemaExecute = dbConfigPasswordGetResponseSchemaExecute;
exports.dbConfigPasswordGetResponseSchemaExecuteTransform = dbConfigPasswordGetResponseSchemaExecuteTransform;
exports.dbConfigPasswordSetExecute = dbConfigPasswordSetExecute;
exports.dbConfigPasswordSetExecuteTransform = dbConfigPasswordSetExecuteTransform;
exports.dbConfigPasswordSetRequestSchemaExecute = dbConfigPasswordSetRequestSchemaExecute;
exports.dbConfigPasswordSetRequestSchemaExecuteTransform = dbConfigPasswordSetRequestSchemaExecuteTransform;
exports.dbConfigPasswordSetResponseSchemaExecute = dbConfigPasswordSetResponseSchemaExecute;
exports.dbConfigPasswordSetResponseSchemaExecuteTransform = dbConfigPasswordSetResponseSchemaExecuteTransform;
exports.dbConfigUserGetExecute = dbConfigUserGetExecute;
exports.dbConfigUserGetExecuteTransform = dbConfigUserGetExecuteTransform;
exports.dbConfigUserGetRequestSchemaExecute = dbConfigUserGetRequestSchemaExecute;
exports.dbConfigUserGetRequestSchemaExecuteTransform = dbConfigUserGetRequestSchemaExecuteTransform;
exports.dbConfigUserGetResponseSchemaExecute = dbConfigUserGetResponseSchemaExecute;
exports.dbConfigUserGetResponseSchemaExecuteTransform = dbConfigUserGetResponseSchemaExecuteTransform;
exports.dbConfigUserSetExecute = dbConfigUserSetExecute;
exports.dbConfigUserSetExecuteTransform = dbConfigUserSetExecuteTransform;
exports.dbConfigUserSetRequestSchemaExecute = dbConfigUserSetRequestSchemaExecute;
exports.dbConfigUserSetRequestSchemaExecuteTransform = dbConfigUserSetRequestSchemaExecuteTransform;
exports.dbConfigUserSetResponseSchemaExecute = dbConfigUserSetResponseSchemaExecute;
exports.dbConfigUserSetResponseSchemaExecuteTransform = dbConfigUserSetResponseSchemaExecuteTransform;
exports.dbKillExecute = dbKillExecute;
exports.dbKillExecuteTransform = dbKillExecuteTransform;
exports.dbKillRequestSchemaExecute = dbKillRequestSchemaExecute;
exports.dbKillRequestSchemaExecuteTransform = dbKillRequestSchemaExecuteTransform;
exports.dbKillResponseSchemaExecute = dbKillResponseSchemaExecute;
exports.dbKillResponseSchemaExecuteTransform = dbKillResponseSchemaExecuteTransform;
exports.dbQueryExecute = dbQueryExecute;
exports.dbQueryExecuteTransform = dbQueryExecuteTransform;
exports.dbQueryRequestSchemaExecute = dbQueryRequestSchemaExecute;
exports.dbQueryRequestSchemaExecuteTransform = dbQueryRequestSchemaExecuteTransform;
exports.dbQueryResponseSchemaExecute = dbQueryResponseSchemaExecute;
exports.dbQueryResponseSchemaExecuteTransform = dbQueryResponseSchemaExecuteTransform;
exports.dbSpawnExecute = dbSpawnExecute;
exports.dbSpawnExecuteTransform = dbSpawnExecuteTransform;
exports.dbSpawnRequestSchemaExecute = dbSpawnRequestSchemaExecute;
exports.dbSpawnRequestSchemaExecuteTransform = dbSpawnRequestSchemaExecuteTransform;
exports.dbSpawnResponseSchemaExecute = dbSpawnResponseSchemaExecute;
exports.dbSpawnResponseSchemaExecuteTransform = dbSpawnResponseSchemaExecuteTransform;
exports.functionsExecuteStandardExecute = functionsExecuteStandardExecute;
exports.functionsExecuteStandardExecuteStreaming = functionsExecuteStandardExecuteStreaming;
exports.functionsExecuteStandardExecuteStreamingTransform = functionsExecuteStandardExecuteStreamingTransform;
exports.functionsExecuteStandardExecuteTransform = functionsExecuteStandardExecuteTransform;
exports.functionsExecuteStandardRequestSchemaExecute = functionsExecuteStandardRequestSchemaExecute;
exports.functionsExecuteStandardRequestSchemaExecuteTransform = functionsExecuteStandardRequestSchemaExecuteTransform;
exports.functionsExecuteStandardResponseSchemaExecute = functionsExecuteStandardResponseSchemaExecute;
exports.functionsExecuteStandardResponseSchemaExecuteTransform = functionsExecuteStandardResponseSchemaExecuteTransform;
exports.functionsExecuteSwissSystemExecute = functionsExecuteSwissSystemExecute;
exports.functionsExecuteSwissSystemExecuteStreaming = functionsExecuteSwissSystemExecuteStreaming;
exports.functionsExecuteSwissSystemExecuteStreamingTransform = functionsExecuteSwissSystemExecuteStreamingTransform;
exports.functionsExecuteSwissSystemExecuteTransform = functionsExecuteSwissSystemExecuteTransform;
exports.functionsExecuteSwissSystemRequestSchemaExecute = functionsExecuteSwissSystemRequestSchemaExecute;
exports.functionsExecuteSwissSystemRequestSchemaExecuteTransform = functionsExecuteSwissSystemRequestSchemaExecuteTransform;
exports.functionsExecuteSwissSystemResponseSchemaExecute = functionsExecuteSwissSystemResponseSchemaExecute;
exports.functionsExecuteSwissSystemResponseSchemaExecuteTransform = functionsExecuteSwissSystemResponseSchemaExecuteTransform;
exports.functionsGetExecute = functionsGetExecute;
exports.functionsGetExecuteTransform = functionsGetExecuteTransform;
exports.functionsGetRequestSchemaExecute = functionsGetRequestSchemaExecute;
exports.functionsGetRequestSchemaExecuteTransform = functionsGetRequestSchemaExecuteTransform;
exports.functionsGetResponseSchemaExecute = functionsGetResponseSchemaExecute;
exports.functionsGetResponseSchemaExecuteTransform = functionsGetResponseSchemaExecuteTransform;
exports.functionsListExecute = functionsListExecute;
exports.functionsListExecuteTransform = functionsListExecuteTransform;
exports.functionsListRequestSchemaExecute = functionsListRequestSchemaExecute;
exports.functionsListRequestSchemaExecuteTransform = functionsListRequestSchemaExecuteTransform;
exports.functionsListResponseSchemaExecute = functionsListResponseSchemaExecute;
exports.functionsListResponseSchemaExecuteTransform = functionsListResponseSchemaExecuteTransform;
exports.functionsProfilesGetExecute = functionsProfilesGetExecute;
exports.functionsProfilesGetExecuteTransform = functionsProfilesGetExecuteTransform;
exports.functionsProfilesGetRequestSchemaExecute = functionsProfilesGetRequestSchemaExecute;
exports.functionsProfilesGetRequestSchemaExecuteTransform = functionsProfilesGetRequestSchemaExecuteTransform;
exports.functionsProfilesGetResponseSchemaExecute = functionsProfilesGetResponseSchemaExecute;
exports.functionsProfilesGetResponseSchemaExecuteTransform = functionsProfilesGetResponseSchemaExecuteTransform;
exports.functionsProfilesListExecute = functionsProfilesListExecute;
exports.functionsProfilesListExecuteTransform = functionsProfilesListExecuteTransform;
exports.functionsProfilesListRequestSchemaExecute = functionsProfilesListRequestSchemaExecute;
exports.functionsProfilesListRequestSchemaExecuteTransform = functionsProfilesListRequestSchemaExecuteTransform;
exports.functionsProfilesListResponseSchemaExecute = functionsProfilesListResponseSchemaExecute;
exports.functionsProfilesListResponseSchemaExecuteTransform = functionsProfilesListResponseSchemaExecuteTransform;
exports.functionsProfilesPublishExecute = functionsProfilesPublishExecute;
exports.functionsProfilesPublishExecuteTransform = functionsProfilesPublishExecuteTransform;
exports.functionsProfilesPublishRequestSchemaExecute = functionsProfilesPublishRequestSchemaExecute;
exports.functionsProfilesPublishRequestSchemaExecuteTransform = functionsProfilesPublishRequestSchemaExecuteTransform;
exports.functionsProfilesPublishResponseSchemaExecute = functionsProfilesPublishResponseSchemaExecute;
exports.functionsProfilesPublishResponseSchemaExecuteTransform = functionsProfilesPublishResponseSchemaExecuteTransform;
exports.functionsPublishExecute = functionsPublishExecute;
exports.functionsPublishExecuteTransform = functionsPublishExecuteTransform;
exports.functionsPublishRequestSchemaExecute = functionsPublishRequestSchemaExecute;
exports.functionsPublishRequestSchemaExecuteTransform = functionsPublishRequestSchemaExecuteTransform;
exports.functionsPublishResponseSchemaExecute = functionsPublishResponseSchemaExecute;
exports.functionsPublishResponseSchemaExecuteTransform = functionsPublishResponseSchemaExecuteTransform;
exports.invokeCliRequest = invokeCliRequest;
exports.listen = listen2;
exports.mcpConfigAddressGetExecute = mcpConfigAddressGetExecute;
exports.mcpConfigAddressGetExecuteTransform = mcpConfigAddressGetExecuteTransform;
exports.mcpConfigAddressGetRequestSchemaExecute = mcpConfigAddressGetRequestSchemaExecute;
exports.mcpConfigAddressGetRequestSchemaExecuteTransform = mcpConfigAddressGetRequestSchemaExecuteTransform;
exports.mcpConfigAddressGetResponseSchemaExecute = mcpConfigAddressGetResponseSchemaExecute;
exports.mcpConfigAddressGetResponseSchemaExecuteTransform = mcpConfigAddressGetResponseSchemaExecuteTransform;
exports.mcpConfigAddressSetExecute = mcpConfigAddressSetExecute;
exports.mcpConfigAddressSetExecuteTransform = mcpConfigAddressSetExecuteTransform;
exports.mcpConfigAddressSetRequestSchemaExecute = mcpConfigAddressSetRequestSchemaExecute;
exports.mcpConfigAddressSetRequestSchemaExecuteTransform = mcpConfigAddressSetRequestSchemaExecuteTransform;
exports.mcpConfigAddressSetResponseSchemaExecute = mcpConfigAddressSetResponseSchemaExecute;
exports.mcpConfigAddressSetResponseSchemaExecuteTransform = mcpConfigAddressSetResponseSchemaExecuteTransform;
exports.mcpConfigGetExecute = mcpConfigGetExecute;
exports.mcpConfigGetExecuteTransform = mcpConfigGetExecuteTransform;
exports.mcpConfigGetRequestSchemaExecute = mcpConfigGetRequestSchemaExecute;
exports.mcpConfigGetRequestSchemaExecuteTransform = mcpConfigGetRequestSchemaExecuteTransform;
exports.mcpConfigGetResponseSchemaExecute = mcpConfigGetResponseSchemaExecute;
exports.mcpConfigGetResponseSchemaExecuteTransform = mcpConfigGetResponseSchemaExecuteTransform;
exports.mcpConfigPortGetExecute = mcpConfigPortGetExecute;
exports.mcpConfigPortGetExecuteTransform = mcpConfigPortGetExecuteTransform;
exports.mcpConfigPortGetRequestSchemaExecute = mcpConfigPortGetRequestSchemaExecute;
exports.mcpConfigPortGetRequestSchemaExecuteTransform = mcpConfigPortGetRequestSchemaExecuteTransform;
exports.mcpConfigPortGetResponseSchemaExecute = mcpConfigPortGetResponseSchemaExecute;
exports.mcpConfigPortGetResponseSchemaExecuteTransform = mcpConfigPortGetResponseSchemaExecuteTransform;
exports.mcpConfigPortSetExecute = mcpConfigPortSetExecute;
exports.mcpConfigPortSetExecuteTransform = mcpConfigPortSetExecuteTransform;
exports.mcpConfigPortSetRequestSchemaExecute = mcpConfigPortSetRequestSchemaExecute;
exports.mcpConfigPortSetRequestSchemaExecuteTransform = mcpConfigPortSetRequestSchemaExecuteTransform;
exports.mcpConfigPortSetResponseSchemaExecute = mcpConfigPortSetResponseSchemaExecute;
exports.mcpConfigPortSetResponseSchemaExecuteTransform = mcpConfigPortSetResponseSchemaExecuteTransform;
exports.mcpKillExecute = mcpKillExecute;
exports.mcpKillExecuteTransform = mcpKillExecuteTransform;
exports.mcpKillRequestSchemaExecute = mcpKillRequestSchemaExecute;
exports.mcpKillRequestSchemaExecuteTransform = mcpKillRequestSchemaExecuteTransform;
exports.mcpKillResponseSchemaExecute = mcpKillResponseSchemaExecute;
exports.mcpKillResponseSchemaExecuteTransform = mcpKillResponseSchemaExecuteTransform;
exports.mcpSpawnExecute = mcpSpawnExecute;
exports.mcpSpawnExecuteTransform = mcpSpawnExecuteTransform;
exports.mcpSpawnRequestSchemaExecute = mcpSpawnRequestSchemaExecute;
exports.mcpSpawnRequestSchemaExecuteTransform = mcpSpawnRequestSchemaExecuteTransform;
exports.mcpSpawnResponseSchemaExecute = mcpSpawnResponseSchemaExecute;
exports.mcpSpawnResponseSchemaExecuteTransform = mcpSpawnResponseSchemaExecuteTransform;
exports.pluginsGetExecute = pluginsGetExecute;
exports.pluginsGetExecuteTransform = pluginsGetExecuteTransform;
exports.pluginsGetRequestSchemaExecute = pluginsGetRequestSchemaExecute;
exports.pluginsGetRequestSchemaExecuteTransform = pluginsGetRequestSchemaExecuteTransform;
exports.pluginsGetResponseSchemaExecute = pluginsGetResponseSchemaExecute;
exports.pluginsGetResponseSchemaExecuteTransform = pluginsGetResponseSchemaExecuteTransform;
exports.pluginsInstallFilesystemExecute = pluginsInstallFilesystemExecute;
exports.pluginsInstallFilesystemExecuteTransform = pluginsInstallFilesystemExecuteTransform;
exports.pluginsInstallFilesystemRequestSchemaExecute = pluginsInstallFilesystemRequestSchemaExecute;
exports.pluginsInstallFilesystemRequestSchemaExecuteTransform = pluginsInstallFilesystemRequestSchemaExecuteTransform;
exports.pluginsInstallFilesystemResponseSchemaExecute = pluginsInstallFilesystemResponseSchemaExecute;
exports.pluginsInstallFilesystemResponseSchemaExecuteTransform = pluginsInstallFilesystemResponseSchemaExecuteTransform;
exports.pluginsInstallGithubExecute = pluginsInstallGithubExecute;
exports.pluginsInstallGithubExecuteTransform = pluginsInstallGithubExecuteTransform;
exports.pluginsInstallGithubRequestSchemaExecute = pluginsInstallGithubRequestSchemaExecute;
exports.pluginsInstallGithubRequestSchemaExecuteTransform = pluginsInstallGithubRequestSchemaExecuteTransform;
exports.pluginsInstallGithubResponseSchemaExecute = pluginsInstallGithubResponseSchemaExecute;
exports.pluginsInstallGithubResponseSchemaExecuteTransform = pluginsInstallGithubResponseSchemaExecuteTransform;
exports.pluginsListExecute = pluginsListExecute;
exports.pluginsListExecuteTransform = pluginsListExecuteTransform;
exports.pluginsListRequestSchemaExecute = pluginsListRequestSchemaExecute;
exports.pluginsListRequestSchemaExecuteTransform = pluginsListRequestSchemaExecuteTransform;
exports.pluginsListResponseSchemaExecute = pluginsListResponseSchemaExecute;
exports.pluginsListResponseSchemaExecuteTransform = pluginsListResponseSchemaExecuteTransform;
exports.pluginsRunExecute = pluginsRunExecute;
exports.pluginsRunExecuteTransform = pluginsRunExecuteTransform;
exports.pluginsRunRequestSchemaExecute = pluginsRunRequestSchemaExecute;
exports.pluginsRunRequestSchemaExecuteTransform = pluginsRunRequestSchemaExecuteTransform;
exports.pluginsRunResponseSchemaExecute = pluginsRunResponseSchemaExecute;
exports.pluginsRunResponseSchemaExecuteTransform = pluginsRunResponseSchemaExecuteTransform;
exports.swarmsGetExecute = swarmsGetExecute;
exports.swarmsGetExecuteTransform = swarmsGetExecuteTransform;
exports.swarmsGetRequestSchemaExecute = swarmsGetRequestSchemaExecute;
exports.swarmsGetRequestSchemaExecuteTransform = swarmsGetRequestSchemaExecuteTransform;
exports.swarmsGetResponseSchemaExecute = swarmsGetResponseSchemaExecute;
exports.swarmsGetResponseSchemaExecuteTransform = swarmsGetResponseSchemaExecuteTransform;
exports.swarmsListExecute = swarmsListExecute;
exports.swarmsListExecuteTransform = swarmsListExecuteTransform;
exports.swarmsListRequestSchemaExecute = swarmsListRequestSchemaExecute;
exports.swarmsListRequestSchemaExecuteTransform = swarmsListRequestSchemaExecuteTransform;
exports.swarmsListResponseSchemaExecute = swarmsListResponseSchemaExecute;
exports.swarmsListResponseSchemaExecuteTransform = swarmsListResponseSchemaExecuteTransform;
exports.swarmsPublishExecute = swarmsPublishExecute;
exports.swarmsPublishExecuteTransform = swarmsPublishExecuteTransform;
exports.swarmsPublishRequestSchemaExecute = swarmsPublishRequestSchemaExecute;
exports.swarmsPublishRequestSchemaExecuteTransform = swarmsPublishRequestSchemaExecuteTransform;
exports.swarmsPublishResponseSchemaExecute = swarmsPublishResponseSchemaExecute;
exports.swarmsPublishResponseSchemaExecuteTransform = swarmsPublishResponseSchemaExecuteTransform;
exports.tasksListExecute = tasksListExecute;
exports.tasksListExecuteTransform = tasksListExecuteTransform;
exports.tasksListRequestSchemaExecute = tasksListRequestSchemaExecute;
exports.tasksListRequestSchemaExecuteTransform = tasksListRequestSchemaExecuteTransform;
exports.tasksListResponseSchemaExecute = tasksListResponseSchemaExecute;
exports.tasksListResponseSchemaExecuteTransform = tasksListResponseSchemaExecuteTransform;
exports.tasksRunExecute = tasksRunExecute;
exports.tasksRunExecuteTransform = tasksRunExecuteTransform;
exports.tasksRunRequestSchemaExecute = tasksRunRequestSchemaExecute;
exports.tasksRunRequestSchemaExecuteTransform = tasksRunRequestSchemaExecuteTransform;
exports.tasksRunResponseSchemaExecute = tasksRunResponseSchemaExecute;
exports.tasksRunResponseSchemaExecuteTransform = tasksRunResponseSchemaExecuteTransform;
exports.tasksScheduleExecute = tasksScheduleExecute;
exports.tasksScheduleExecuteTransform = tasksScheduleExecuteTransform;
exports.tasksScheduleRequestSchemaExecute = tasksScheduleRequestSchemaExecute;
exports.tasksScheduleRequestSchemaExecuteTransform = tasksScheduleRequestSchemaExecuteTransform;
exports.tasksScheduleResponseSchemaExecute = tasksScheduleResponseSchemaExecute;
exports.tasksScheduleResponseSchemaExecuteTransform = tasksScheduleResponseSchemaExecuteTransform;
exports.toolsGetExecute = toolsGetExecute;
exports.toolsGetExecuteTransform = toolsGetExecuteTransform;
exports.toolsGetRequestSchemaExecute = toolsGetRequestSchemaExecute;
exports.toolsGetRequestSchemaExecuteTransform = toolsGetRequestSchemaExecuteTransform;
exports.toolsGetResponseSchemaExecute = toolsGetResponseSchemaExecute;
exports.toolsGetResponseSchemaExecuteTransform = toolsGetResponseSchemaExecuteTransform;
exports.toolsInstallExecute = toolsInstallExecute;
exports.toolsInstallExecuteTransform = toolsInstallExecuteTransform;
exports.toolsInstallRequestSchemaExecute = toolsInstallRequestSchemaExecute;
exports.toolsInstallRequestSchemaExecuteTransform = toolsInstallRequestSchemaExecuteTransform;
exports.toolsInstallResponseSchemaExecute = toolsInstallResponseSchemaExecute;
exports.toolsInstallResponseSchemaExecuteTransform = toolsInstallResponseSchemaExecuteTransform;
exports.toolsListExecute = toolsListExecute;
exports.toolsListExecuteTransform = toolsListExecuteTransform;
exports.toolsListRequestSchemaExecute = toolsListRequestSchemaExecute;
exports.toolsListRequestSchemaExecuteTransform = toolsListRequestSchemaExecuteTransform;
exports.toolsListResponseSchemaExecute = toolsListResponseSchemaExecute;
exports.toolsListResponseSchemaExecuteTransform = toolsListResponseSchemaExecuteTransform;
exports.toolsRunExecute = toolsRunExecute;
exports.toolsRunExecuteTransform = toolsRunExecuteTransform;
exports.toolsRunRequestSchemaExecute = toolsRunRequestSchemaExecute;
exports.toolsRunRequestSchemaExecuteTransform = toolsRunRequestSchemaExecuteTransform;
exports.toolsRunResponseSchemaExecute = toolsRunResponseSchemaExecute;
exports.toolsRunResponseSchemaExecuteTransform = toolsRunResponseSchemaExecuteTransform;
exports.updateExecute = updateExecute;
exports.updateExecuteTransform = updateExecuteTransform;
exports.updateRequestSchemaExecute = updateRequestSchemaExecute;
exports.updateRequestSchemaExecuteTransform = updateRequestSchemaExecuteTransform;
exports.updateResponseSchemaExecute = updateResponseSchemaExecute;
exports.updateResponseSchemaExecuteTransform = updateResponseSchemaExecuteTransform;
exports.viewerConfigAddressGetExecute = viewerConfigAddressGetExecute;
exports.viewerConfigAddressGetExecuteTransform = viewerConfigAddressGetExecuteTransform;
exports.viewerConfigAddressGetRequestSchemaExecute = viewerConfigAddressGetRequestSchemaExecute;
exports.viewerConfigAddressGetRequestSchemaExecuteTransform = viewerConfigAddressGetRequestSchemaExecuteTransform;
exports.viewerConfigAddressGetResponseSchemaExecute = viewerConfigAddressGetResponseSchemaExecute;
exports.viewerConfigAddressGetResponseSchemaExecuteTransform = viewerConfigAddressGetResponseSchemaExecuteTransform;
exports.viewerConfigAddressSetExecute = viewerConfigAddressSetExecute;
exports.viewerConfigAddressSetExecuteTransform = viewerConfigAddressSetExecuteTransform;
exports.viewerConfigAddressSetRequestSchemaExecute = viewerConfigAddressSetRequestSchemaExecute;
exports.viewerConfigAddressSetRequestSchemaExecuteTransform = viewerConfigAddressSetRequestSchemaExecuteTransform;
exports.viewerConfigAddressSetResponseSchemaExecute = viewerConfigAddressSetResponseSchemaExecute;
exports.viewerConfigAddressSetResponseSchemaExecuteTransform = viewerConfigAddressSetResponseSchemaExecuteTransform;
exports.viewerConfigGetExecute = viewerConfigGetExecute;
exports.viewerConfigGetExecuteTransform = viewerConfigGetExecuteTransform;
exports.viewerConfigGetRequestSchemaExecute = viewerConfigGetRequestSchemaExecute;
exports.viewerConfigGetRequestSchemaExecuteTransform = viewerConfigGetRequestSchemaExecuteTransform;
exports.viewerConfigGetResponseSchemaExecute = viewerConfigGetResponseSchemaExecute;
exports.viewerConfigGetResponseSchemaExecuteTransform = viewerConfigGetResponseSchemaExecuteTransform;
exports.viewerConfigSecretGetExecute = viewerConfigSecretGetExecute;
exports.viewerConfigSecretGetExecuteTransform = viewerConfigSecretGetExecuteTransform;
exports.viewerConfigSecretGetRequestSchemaExecute = viewerConfigSecretGetRequestSchemaExecute;
exports.viewerConfigSecretGetRequestSchemaExecuteTransform = viewerConfigSecretGetRequestSchemaExecuteTransform;
exports.viewerConfigSecretGetResponseSchemaExecute = viewerConfigSecretGetResponseSchemaExecute;
exports.viewerConfigSecretGetResponseSchemaExecuteTransform = viewerConfigSecretGetResponseSchemaExecuteTransform;
exports.viewerConfigSecretSetExecute = viewerConfigSecretSetExecute;
exports.viewerConfigSecretSetExecuteTransform = viewerConfigSecretSetExecuteTransform;
exports.viewerConfigSecretSetRequestSchemaExecute = viewerConfigSecretSetRequestSchemaExecute;
exports.viewerConfigSecretSetRequestSchemaExecuteTransform = viewerConfigSecretSetRequestSchemaExecuteTransform;
exports.viewerConfigSecretSetResponseSchemaExecute = viewerConfigSecretSetResponseSchemaExecute;
exports.viewerConfigSecretSetResponseSchemaExecuteTransform = viewerConfigSecretSetResponseSchemaExecuteTransform;
exports.viewerConfigSignatureGetExecute = viewerConfigSignatureGetExecute;
exports.viewerConfigSignatureGetExecuteTransform = viewerConfigSignatureGetExecuteTransform;
exports.viewerConfigSignatureGetRequestSchemaExecute = viewerConfigSignatureGetRequestSchemaExecute;
exports.viewerConfigSignatureGetRequestSchemaExecuteTransform = viewerConfigSignatureGetRequestSchemaExecuteTransform;
exports.viewerConfigSignatureGetResponseSchemaExecute = viewerConfigSignatureGetResponseSchemaExecute;
exports.viewerConfigSignatureGetResponseSchemaExecuteTransform = viewerConfigSignatureGetResponseSchemaExecuteTransform;
exports.viewerConfigSignatureSetExecute = viewerConfigSignatureSetExecute;
exports.viewerConfigSignatureSetExecuteTransform = viewerConfigSignatureSetExecuteTransform;
exports.viewerConfigSignatureSetRequestSchemaExecute = viewerConfigSignatureSetRequestSchemaExecute;
exports.viewerConfigSignatureSetRequestSchemaExecuteTransform = viewerConfigSignatureSetRequestSchemaExecuteTransform;
exports.viewerConfigSignatureSetResponseSchemaExecute = viewerConfigSignatureSetResponseSchemaExecute;
exports.viewerConfigSignatureSetResponseSchemaExecuteTransform = viewerConfigSignatureSetResponseSchemaExecuteTransform;
exports.viewerGenerateSecretSignaturePairExecute = viewerGenerateSecretSignaturePairExecute;
exports.viewerGenerateSecretSignaturePairExecuteTransform = viewerGenerateSecretSignaturePairExecuteTransform;
exports.viewerGenerateSecretSignaturePairRequestSchemaExecute = viewerGenerateSecretSignaturePairRequestSchemaExecute;
exports.viewerGenerateSecretSignaturePairRequestSchemaExecuteTransform = viewerGenerateSecretSignaturePairRequestSchemaExecuteTransform;
exports.viewerGenerateSecretSignaturePairResponseSchemaExecute = viewerGenerateSecretSignaturePairResponseSchemaExecute;
exports.viewerGenerateSecretSignaturePairResponseSchemaExecuteTransform = viewerGenerateSecretSignaturePairResponseSchemaExecuteTransform;
exports.viewerKillExecute = viewerKillExecute;
exports.viewerKillExecuteTransform = viewerKillExecuteTransform;
exports.viewerKillRequestSchemaExecute = viewerKillRequestSchemaExecute;
exports.viewerKillRequestSchemaExecuteTransform = viewerKillRequestSchemaExecuteTransform;
exports.viewerKillResponseSchemaExecute = viewerKillResponseSchemaExecute;
exports.viewerKillResponseSchemaExecuteTransform = viewerKillResponseSchemaExecuteTransform;
exports.viewerSendExecute = viewerSendExecute;
exports.viewerSendExecuteTransform = viewerSendExecuteTransform;
exports.viewerSendRequestSchemaExecute = viewerSendRequestSchemaExecute;
exports.viewerSendRequestSchemaExecuteTransform = viewerSendRequestSchemaExecuteTransform;
exports.viewerSendResponseSchemaExecute = viewerSendResponseSchemaExecute;
exports.viewerSendResponseSchemaExecuteTransform = viewerSendResponseSchemaExecuteTransform;
exports.viewerSpawnExecute = viewerSpawnExecute;
exports.viewerSpawnExecuteTransform = viewerSpawnExecuteTransform;
exports.viewerSpawnRequestSchemaExecute = viewerSpawnRequestSchemaExecute;
exports.viewerSpawnRequestSchemaExecuteTransform = viewerSpawnRequestSchemaExecuteTransform;
exports.viewerSpawnResponseSchemaExecute = viewerSpawnResponseSchemaExecute;
exports.viewerSpawnResponseSchemaExecuteTransform = viewerSpawnResponseSchemaExecuteTransform;
