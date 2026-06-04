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
var CliCommandAgentsListActiveResponseItemSchema = zod.z.object({
  agent_id: zod.z.string(),
  last_log: zod.z.number().int().min(0).max(18446744073709552e3)
}).meta({ title: "cli.command.agents.list.active.ResponseItem" });

// src/viewer/command/agents/list/active.ts
function agentsListActiveExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/list/active" }), zod.z.union([CliErrorSchema, CliCommandAgentsListActiveResponseItemSchema]));
}
function agentsListActiveExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/list/active" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function agentsListActiveRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/list/active/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents list active request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsListActiveRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/list/active/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents list active request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsListActiveResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/list/active/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents list active response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsListActiveResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/list/active/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents list active response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandAgentsListAvailableResponseFavoriteSchema = zod.z.union([zod.z.object({
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
}).meta({ "variantTitle": "Mock" })]).meta({ title: "cli.command.agents.list.available.ResponseFavorite" });

// src/cli/command/agents/list/available/responseItem.ts
var CliCommandAgentsListAvailableResponseItemSchema = zod.z.union([CliCommandAgentsListAvailableResponseFavoriteSchema.meta({ "title": "cli.command.agents.list.available.ResponseFavorite", "variantTitle": "Favorite" }), RemotePathSchema.meta({ "title": "RemotePath", "variantTitle": "Item" })]).meta({ title: "cli.command.agents.list.available.ResponseItem" });

// src/viewer/command/agents/list/available.ts
function agentsListAvailableExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/list/available" }), zod.z.union([CliErrorSchema, CliCommandAgentsListAvailableResponseItemSchema]));
}
function agentsListAvailableExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/list/available" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function agentsListAvailableRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/list/available/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents list available request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsListAvailableRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/list/available/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents list available request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsListAvailableResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/list/available/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents list available response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsListAvailableResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/list/available/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents list available response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandAgentsMeResponseSchema = zod.z.object({
  agent_instance_hierarchy: zod.z.string()
}).meta({ title: "cli.command.agents.me.Response" });

// src/viewer/command/agents/me.ts
async function agentsMeExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/me" }), zod.z.union([CliErrorSchema, CliCommandAgentsMeResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents me: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMeExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/me" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents me: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMeRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/me/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents me request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMeRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/me/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents me request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMeResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/me/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents me response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMeResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/me/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents me response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandAgentsMessageResponseSchema = zod.z.union([zod.z.object({
  agent_instance_hierarchy: zod.z.string(),
  response_id: zod.z.string()
}).meta({ "variantTitle": "Queued" }), zod.z.object({
  agent_instance_hierarchy: zod.z.string()
}).meta({ "variantTitle": "Delivered" })]).meta({ title: "cli.command.agents.message.Response" });

// src/viewer/command/agents/message.ts
async function agentsMessageExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/message" }), zod.z.union([CliErrorSchema, CliCommandAgentsMessageResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents message: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMessageExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/message" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
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
var CliCommandAgentsReadAllResponseContentSchema = zod.z.union([zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3).meta({ "variantTitle": "One" }), zod.z.array(zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3)).meta({ "variantTitle": "Many" })]).meta({ title: "cli.command.agents.read.all.ResponseContent" });
var CliCommandAgentsReadAllResponseQueueMessageSchema = zod.z.union([zod.z.object({
  content: CliCommandAgentsReadAllResponseContentSchema,
  name: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  type: zod.z.literal("developer")
}).meta({ "variantTitle": "Developer" }), zod.z.object({
  content: CliCommandAgentsReadAllResponseContentSchema,
  name: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  type: zod.z.literal("system")
}).meta({ "variantTitle": "System" }), zod.z.object({
  content: CliCommandAgentsReadAllResponseContentSchema,
  name: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  type: zod.z.literal("user")
}).meta({ "variantTitle": "User" }), zod.z.object({
  content: CliCommandAgentsReadAllResponseContentSchema.nullable().meta({ omitempty: true }).optional(),
  name: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  reasoning: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3).nullable().meta({ omitempty: true }).optional(),
  refusal: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3).nullable().meta({ omitempty: true }).optional(),
  tool_calls: zod.z.array(zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3)).nullable().meta({ omitempty: true }).optional(),
  type: zod.z.literal("assistant")
}).meta({ "variantTitle": "Assistant" }), zod.z.object({
  content: CliCommandAgentsReadAllResponseContentSchema,
  tool_call_id: zod.z.string(),
  type: zod.z.literal("tool")
}).meta({ "variantTitle": "Tool" })]).meta({ title: "cli.command.agents.read.all.ResponseQueueMessage" });

// src/cli/command/agents/read/all/responseQueueItem.ts
var CliCommandAgentsReadAllResponseQueueItemSchema = zod.z.union([zod.z.object({
  content: CliCommandAgentsReadAllResponseContentSchema.nullable().meta({ omitempty: true }).optional(),
  reasoning: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3).nullable().meta({ omitempty: true }).optional(),
  refusal: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3).nullable().meta({ omitempty: true }).optional(),
  tool_calls: zod.z.array(zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3)).nullable().meta({ omitempty: true }).optional(),
  type: zod.z.literal("assistant_response")
}).meta({ "variantTitle": "AssistantResponse" }), zod.z.object({
  content: CliCommandAgentsReadAllResponseContentSchema,
  tool_call_id: zod.z.string(),
  type: zod.z.literal("tool_response")
}).meta({ "variantTitle": "ToolResponse" }), zod.z.object({
  content: CliCommandAgentsReadAllResponseContentSchema,
  type: zod.z.literal("notification")
}).meta({ "variantTitle": "Notification" }), zod.z.object({
  messages: zod.z.array(CliCommandAgentsReadAllResponseQueueMessageSchema),
  type: zod.z.literal("agent_completion_request")
}).meta({ "variantTitle": "AgentCompletionRequest" }), zod.z.object({
  id: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  type: zod.z.literal("function_execution_request")
}).meta({ "variantTitle": "FunctionExecutionRequest" }), zod.z.object({
  id: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  type: zod.z.literal("function_invention_recursive_request")
}).meta({ "variantTitle": "FunctionInventionRecursiveRequest" })]).meta({ title: "cli.command.agents.read.all.ResponseQueueItem" });

// src/cli/command/agents/read/all/responseItem.ts
var CliCommandAgentsReadAllResponseItemSchema = zod.z.object({
  agent_id: zod.z.string(),
  items: zod.z.array(CliCommandAgentsReadAllResponseQueueItemSchema)
}).meta({ title: "cli.command.agents.read.all.ResponseItem" });

// src/viewer/command/agents/read/all.ts
function agentsReadAllExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/read/all" }), zod.z.union([CliErrorSchema, CliCommandAgentsReadAllResponseItemSchema]));
}
function agentsReadAllExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/read/all" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function agentsReadAllRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/read/all/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents read all request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsReadAllRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/read/all/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents read all request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsReadAllResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/read/all/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents read all response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsReadAllResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/read/all/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents read all response_schema: cli produced no output before the end marker");
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
var LogReferenceTagSchema = zod.z.literal("reference").describe('Constant `"reference"` discriminator \u2014 the `"type"` field on every\n`LogReference` variant.').meta({ title: "LogReferenceTag" });

// src/logReference.ts
var LogReferenceSchema = zod.z.object({
  path: zod.z.string().describe("Relative on-disk path of the referenced file (under\n`${config_base_dir}/logs/`). Skipped when empty \u2014 the no-data\nsentinel case used by some wrappers when the inner chunk has\nno content to log.").meta({ omitempty: true }),
  type: LogReferenceTagSchema
}).describe("Plain on-disk pointer (`type` + `path` only).").meta({ title: "LogReference" });

// src/agent/completions/message/richContentLog.ts
var AgentCompletionsMessageRichContentLogSchema = zod.z.union([LogReferenceSchema.meta({ "title": "LogReference", "variantTitle": "Reference" }), zod.z.array(LogReferenceSchema).meta({ "variantTitle": "Parts" })]).meta({ title: "agent.completions.message.RichContentLog" });

// src/agent/completions/message/assistantMessageLog.ts
var AgentCompletionsMessageAssistantMessageLogSchema = zod.z.object({
  content: AgentCompletionsMessageRichContentLogSchema.nullable().meta({ omitempty: true }).optional(),
  name: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  reasoning: LogReferenceSchema.nullable().meta({ omitempty: true }).optional(),
  refusal: LogReferenceSchema.nullable().meta({ omitempty: true }).optional(),
  tool_calls: zod.z.array(LogReferenceSchema).nullable().meta({ omitempty: true }).optional()
}).meta({ title: "agent.completions.message.AssistantMessageLog" });
var AgentCompletionsMessageSimpleContentLogSchema = zod.z.union([LogReferenceSchema.meta({ "title": "LogReference", "variantTitle": "Reference" }), zod.z.array(LogReferenceSchema).meta({ "variantTitle": "Parts" })]).meta({ title: "agent.completions.message.SimpleContentLog" });

// src/agent/completions/message/developerMessageLog.ts
var AgentCompletionsMessageDeveloperMessageLogSchema = zod.z.object({
  content: AgentCompletionsMessageSimpleContentLogSchema,
  name: zod.z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "agent.completions.message.DeveloperMessageLog" });
var AgentCompletionsMessageSystemMessageLogSchema = zod.z.object({
  content: AgentCompletionsMessageSimpleContentLogSchema,
  name: zod.z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "agent.completions.message.SystemMessageLog" });
var AgentCompletionsMessageToolResponseMetadataSchema = zod.z.object({
  notifications: zod.z.number().int().min(0).max(18446744073709552e3).nullable().describe("Count of pending notifications the proxy drained and prepended\nto the tool response's `content` before returning. Only set\nwhen at least one notification was drained.").meta({ omitempty: true }).optional()
}).describe("Vendor-extension metadata attached to a tool response. The\n`objectiveai-mcp-proxy` populates known keys (currently\n`notifications`); the SDK lossy-decodes the MCP `_meta` bag into\nthis typed shape. Unknown keys are dropped.").meta({ title: "agent.completions.message.ToolResponseMetadata" });

// src/agent/completions/message/toolMessageLog.ts
var AgentCompletionsMessageToolMessageLogSchema = zod.z.object({
  content: AgentCompletionsMessageRichContentLogSchema,
  metadata: AgentCompletionsMessageToolResponseMetadataSchema.nullable().meta({ omitempty: true }).optional(),
  tool_call_id: zod.z.string()
}).meta({ title: "agent.completions.message.ToolMessageLog" });
var AgentCompletionsMessageUserMessageLogSchema = zod.z.object({
  content: AgentCompletionsMessageRichContentLogSchema,
  name: zod.z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "agent.completions.message.UserMessageLog" });

// src/agent/completions/message/messageLog.ts
var AgentCompletionsMessageMessageLogSchema = zod.z.union([AgentCompletionsMessageDeveloperMessageLogSchema.and(zod.z.object({
  role: zod.z.literal("developer")
})).meta({ "variantTitle": "Developer" }), AgentCompletionsMessageSystemMessageLogSchema.and(zod.z.object({
  role: zod.z.literal("system")
})).meta({ "variantTitle": "System" }), AgentCompletionsMessageUserMessageLogSchema.and(zod.z.object({
  role: zod.z.literal("user")
})).meta({ "variantTitle": "User" }), AgentCompletionsMessageAssistantMessageLogSchema.and(zod.z.object({
  role: zod.z.literal("assistant")
})).meta({ "variantTitle": "Assistant" }), AgentCompletionsMessageToolMessageLogSchema.and(zod.z.object({
  role: zod.z.literal("tool")
})).meta({ "variantTitle": "Tool" })]).meta({ title: "agent.completions.message.MessageLog" });
var AgentCompletionsMessageVideoUrlSchema = zod.z.object({
  url: zod.z.string().describe("The URL of the video.")
}).describe("A video URL for multimodal input.").meta({ title: "agent.completions.message.VideoUrl" });
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

// src/agent/completions/request/agentCompletionCreateParamsLog.ts
var AgentCompletionsRequestAgentCompletionCreateParamsLogSchema = zod.z.object({
  agent: AgentInlineAgentBaseWithFallbacksOrRemoteCommitOptionalSchema,
  continuation: LogReferenceSchema.nullable().meta({ omitempty: true }).optional(),
  messages: zod.z.array(LogReferenceSchema),
  provider: AgentCompletionsRequestProviderSchema.nullable().meta({ omitempty: true }).optional(),
  response_format: LogReferenceSchema.nullable().meta({ omitempty: true }).optional(),
  seed: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3).nullable().meta({ omitempty: true }).optional(),
  stream: zod.z.boolean().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "agent.completions.request.AgentCompletionCreateParamsLog" });
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
var AgentCompletionsResponseStreamingObjectSchema = zod.z.literal("agent.completion.chunk").describe("A agent completion chunk object.").meta({ title: "agent.completions.response.streaming.Object" });
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

// src/agent/completions/response/usage.ts
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

// src/agent/completions/response/streaming/agentCompletionChunkLog.ts
var AgentCompletionsResponseStreamingAgentCompletionChunkLogSchema = zod.z.object({
  agent_full_id: zod.z.string(),
  agent_id: zod.z.string(),
  agent_instance_hierarchy: zod.z.string(),
  agent_remote: RemotePathSchema.nullable().meta({ omitempty: true }).optional(),
  continuation: LogReferenceSchema.nullable().meta({ omitempty: true }).optional(),
  created: zod.z.number().int().min(0).max(18446744073709552e3),
  error: ErrorResponseErrorSchema.nullable().meta({ omitempty: true }).optional(),
  id: zod.z.string(),
  messages: zod.z.array(LogReferenceSchema),
  messages_queued: zod.z.boolean().nullable().meta({ omitempty: true }).optional(),
  object: AgentCompletionsResponseStreamingObjectSchema,
  upstream: AgentUpstreamSchema,
  usage: AgentCompletionsResponseUsageSchema.nullable().meta({ omitempty: true }).optional()
}).meta({ title: "agent.completions.response.streaming.AgentCompletionChunkLog" });
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
var FunctionsExpressionInputValueLogSchema = zod.z.union([LogReferenceSchema.meta({ "title": "LogReference", "variantTitle": "Reference" }), zod.z.record(zod.z.string(), LogReferenceSchema).meta({ "variantTitle": "Object" }), zod.z.array(LogReferenceSchema).meta({ "variantTitle": "Array" })]).meta({ title: "functions.expression.InputValueLog" });
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

// src/functions/executions/request/functionExecutionCreateParamsLog.ts
var FunctionsExecutionsRequestFunctionExecutionCreateParamsLogSchema = zod.z.object({
  continuation: LogReferenceSchema.nullable().meta({ omitempty: true }).optional(),
  from_cache: zod.z.boolean().nullable().meta({ omitempty: true }).optional(),
  function: FunctionsFullInlineFunctionOrRemoteCommitOptionalSchema,
  input: FunctionsExpressionInputValueLogSchema,
  invert: zod.z.boolean().nullable().meta({ omitempty: true }).optional(),
  profile: FunctionsInlineProfileOrRemoteCommitOptionalSchema,
  provider: AgentCompletionsRequestProviderSchema.nullable().meta({ omitempty: true }).optional(),
  reasoning: FunctionsExecutionsRequestReasoningSchema.nullable().meta({ omitempty: true }).optional(),
  retry_token: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  seed: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3).nullable().meta({ omitempty: true }).optional(),
  split: zod.z.boolean().nullable().meta({ omitempty: true }).optional(),
  strategy: FunctionsExecutionsRequestStrategySchema.nullable().meta({ omitempty: true }).optional(),
  stream: zod.z.boolean().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "functions.executions.request.FunctionExecutionCreateParamsLog" });
var FunctionsExpressionTaskOutputSchema = zod.z.union([zod.z.number().min(-34028234663852886e22).max(34028234663852886e22).describe("A single scalar score.").meta({ "variantTitle": "Scalar" }), zod.z.array(zod.z.number().min(-34028234663852886e22).max(34028234663852886e22)).describe("A vector of scores.").meta({ "variantTitle": "Vector" }), zod.z.array(zod.z.array(zod.z.number().min(-34028234663852886e22).max(34028234663852886e22))).describe("Multiple vectors of scores (from mapped tasks).").meta({ "variantTitle": "Vectors" }), zod.z.object({
  error: JsonValueSchema
}).describe("An error occurred during execution.").meta({ "variantTitle": "Err" })]).describe("Owned task output variants.").meta({ title: "functions.expression.TaskOutput" });

// src/functions/executions/response/output.ts
var FunctionsExecutionsResponseOutputSchema = zod.z.object({
  output: FunctionsExpressionTaskOutputSchema
}).describe("Wrapper for function execution output, distinguishing between\na null output value and a missing output.").meta({ title: "functions.executions.response.Output" });
var FunctionsExecutionsResponseStreamingObjectSchema = zod.z.enum(["scalar.function.execution.chunk", "vector.function.execution.chunk"]).meta({ title: "functions.executions.response.streaming.Object" });
var FunctionsExecutionsResponseStreamingReasoningSummaryLogReferenceLogReferenceSchema = zod.z.object({
  error: JsonValueSchema.meta({ omitempty: true }),
  path: zod.z.string().meta({ omitempty: true }),
  type: LogReferenceTagSchema
}).meta({ title: "functions.executions.response.streaming.reasoning_summary_log_reference.LogReference" });
var FunctionsExecutionsResponseStreamingFunctionExecutionTaskLogReferenceLogReferenceSchema = zod.z.object({
  index: zod.z.number().int().min(0).max(18446744073709552e3),
  path: zod.z.string().meta({ omitempty: true }),
  split_index: zod.z.number().int().min(0).max(18446744073709552e3).nullable().meta({ omitempty: true }).optional(),
  swiss_pool_index: zod.z.number().int().min(0).max(18446744073709552e3).nullable().meta({ omitempty: true }).optional(),
  swiss_round: zod.z.number().int().min(0).max(18446744073709552e3).nullable().meta({ omitempty: true }).optional(),
  task_index: zod.z.number().int().min(0).max(18446744073709552e3),
  task_path: zod.z.array(zod.z.number().int().min(0).max(18446744073709552e3)),
  type: LogReferenceTagSchema
}).meta({ title: "functions.executions.response.streaming.function_execution_task_log_reference.LogReference" });
var FunctionsExecutionsResponseStreamingVectorCompletionTaskLogReferenceLogReferenceSchema = zod.z.object({
  error: JsonValueSchema.meta({ omitempty: true }),
  index: zod.z.number().int().min(0).max(18446744073709552e3),
  path: zod.z.string().meta({ omitempty: true }),
  task_index: zod.z.number().int().min(0).max(18446744073709552e3),
  task_path: zod.z.array(zod.z.number().int().min(0).max(18446744073709552e3)),
  type: LogReferenceTagSchema
}).meta({ title: "functions.executions.response.streaming.vector_completion_task_log_reference.LogReference" });

// src/functions/executions/response/streaming/task_log_reference/logReference.ts
var FunctionsExecutionsResponseStreamingTaskLogReferenceLogReferenceSchema = zod.z.union([FunctionsExecutionsResponseStreamingFunctionExecutionTaskLogReferenceLogReferenceSchema.meta({ "title": "functions.executions.response.streaming.function_execution_task_log_reference.LogReference", "variantTitle": "FunctionExecution" }), FunctionsExecutionsResponseStreamingVectorCompletionTaskLogReferenceLogReferenceSchema.meta({ "title": "functions.executions.response.streaming.vector_completion_task_log_reference.LogReference", "variantTitle": "VectorCompletion" })]).meta({ title: "functions.executions.response.streaming.task_log_reference.LogReference" });

// src/functions/executions/response/streaming/functionExecutionChunkLog.ts
var FunctionsExecutionsResponseStreamingFunctionExecutionChunkLogSchema = zod.z.object({
  created: zod.z.number().int().min(0).max(18446744073709552e3),
  error: ErrorResponseErrorSchema.nullable().meta({ omitempty: true }).optional(),
  function: RemotePathSchema.nullable().optional(),
  id: zod.z.string(),
  object: FunctionsExecutionsResponseStreamingObjectSchema,
  output: FunctionsExecutionsResponseOutputSchema.nullable().meta({ omitempty: true }).optional(),
  profile: RemotePathSchema.nullable().optional(),
  reasoning: FunctionsExecutionsResponseStreamingReasoningSummaryLogReferenceLogReferenceSchema.nullable().describe("Appended at the end to match the legacy on-disk order: the\nold code inserted `reasoning` via `Map::insert` AFTER all\nthe shell fields, putting it at the tail of the object.").meta({ omitempty: true }).optional(),
  retry_token: LogReferenceSchema.nullable().meta({ omitempty: true }).optional(),
  tasks: zod.z.array(FunctionsExecutionsResponseStreamingTaskLogReferenceLogReferenceSchema),
  tasks_errors: zod.z.boolean().nullable().meta({ omitempty: true }).optional(),
  usage: AgentCompletionsResponseUsageSchema.nullable().meta({ omitempty: true }).optional()
}).meta({ title: "functions.executions.response.streaming.FunctionExecutionChunkLog" });
var FunctionsInventionsPromptsStepPromptTypeSchema = zod.z.enum(["alpha.scalar.branch.function", "alpha.scalar.leaf.function", "alpha.vector.branch.function", "alpha.vector.leaf.function"]).describe("The type of invention state a prompt applies to.").meta({ title: "functions.inventions.prompts.StepPromptType" });

// src/functions/inventions/prompts/stepPromptExpression.ts
var FunctionsInventionsPromptsStepPromptExpressionSchema = zod.z.object({
  type: zod.z.array(FunctionsInventionsPromptsStepPromptTypeSchema),
  value: zod.z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), zod.z.string().describe("A literal value.").meta({ "variantTitle": "Value" })]).describe('A value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```')
}).describe("A prompt for a single invention step, applicable to one or more state types.").meta({ title: "functions.inventions.prompts.StepPromptExpression" });

// src/functions/inventions/prompts/inlinePrompt.ts
var FunctionsInventionsPromptsInlinePromptSchema = zod.z.object({
  description_step: zod.z.array(FunctionsInventionsPromptsStepPromptExpressionSchema),
  essay_step: zod.z.array(FunctionsInventionsPromptsStepPromptExpressionSchema),
  essay_tasks_step: zod.z.array(FunctionsInventionsPromptsStepPromptExpressionSchema),
  input_schema_step: zod.z.array(FunctionsInventionsPromptsStepPromptExpressionSchema),
  tasks_step: zod.z.array(FunctionsInventionsPromptsStepPromptExpressionSchema)
}).describe("Inline invention prompt configuration for all steps.").meta({ title: "functions.inventions.prompts.InlinePrompt" });

// src/functions/inventions/prompts/inlinePromptOrRemoteCommitOptional.ts
var FunctionsInventionsPromptsInlinePromptOrRemoteCommitOptionalSchema = zod.z.union([FunctionsInventionsPromptsInlinePromptSchema.meta({ "title": "functions.inventions.prompts.InlinePrompt", "variantTitle": "Inline" }), RemotePathCommitOptionalSchema.meta({ "title": "RemotePathCommitOptional", "variantTitle": "Remote" })]).describe("A prompt specification that is either an inline prompt definition\nor a remote path reference.").meta({ title: "functions.inventions.prompts.InlinePromptOrRemoteCommitOptional" });
var RemoteSchema = zod.z.union([zod.z.literal("github").describe("GitHub repository.").meta({ "variantTitle": "Github" }), zod.z.literal("filesystem").describe("Local filesystem.").meta({ "variantTitle": "Filesystem" }), zod.z.literal("mock").describe("Mock (for testing).").meta({ "variantTitle": "Mock" })]).describe("The remote source where a function, profile, or agent is hosted.").meta({ title: "Remote" });

// src/functions/inventions/recursive/request/functionInventionRecursiveCreateParamsLog.ts
var FunctionsInventionsRecursiveRequestFunctionInventionRecursiveCreateParamsLogSchema = zod.z.object({
  agent: AgentInlineAgentBaseWithFallbacksOrRemoteCommitOptionalSchema,
  continuation: LogReferenceSchema.nullable().meta({ omitempty: true }).optional(),
  max_step_retries: zod.z.number().int().min(0).max(4294967295).nullable().meta({ omitempty: true }).optional(),
  overwrite: zod.z.boolean().nullable().meta({ omitempty: true }).optional(),
  prompt: FunctionsInventionsPromptsInlinePromptOrRemoteCommitOptionalSchema,
  provider: AgentCompletionsRequestProviderSchema.nullable().meta({ omitempty: true }).optional(),
  remote: RemoteSchema,
  seed: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3).nullable().meta({ omitempty: true }).optional(),
  state: LogReferenceSchema,
  stream: zod.z.boolean().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "functions.inventions.recursive.request.FunctionInventionRecursiveCreateParamsLog" });
var FunctionsInventionsRecursiveResponseStreamingObjectSchema = zod.z.enum(["alpha.scalar.function.invention.recursive.chunk", "alpha.vector.function.invention.recursive.chunk"]).meta({ title: "functions.inventions.recursive.response.streaming.Object" });
var IndexedLogReferenceSchema = zod.z.object({
  index: zod.z.number().int().min(0).max(18446744073709552e3),
  path: zod.z.string().meta({ omitempty: true }),
  type: LogReferenceTagSchema
}).describe("`LogReference` for log files keyed by an `index` \u2014 used by\nper-agent / per-invention completion wrappers that need to preserve\ntheir position within a parent collection (a vector completion's\nswarm-index, an invention's per-invention index, etc.).").meta({ title: "IndexedLogReference" });

// src/functions/inventions/recursive/response/streaming/functionInventionRecursiveChunkLog.ts
var FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunkLogSchema = zod.z.object({
  created: zod.z.number().int().min(0).max(18446744073709552e3),
  id: zod.z.string(),
  inventions: zod.z.array(IndexedLogReferenceSchema),
  inventions_errors: zod.z.boolean().nullable().meta({ omitempty: true }).optional(),
  object: FunctionsInventionsRecursiveResponseStreamingObjectSchema,
  usage: AgentCompletionsResponseUsageSchema.nullable().meta({ omitempty: true }).optional()
}).meta({ title: "functions.inventions.recursive.response.streaming.FunctionInventionRecursiveChunkLog" });
var FunctionsInventionsStateAlphaScalarBranchStateSchema = zod.z.object({
  depth: zod.z.number().int().min(0).max(18446744073709552e3),
  description: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  essay: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  essay_tasks: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  input_schema: FunctionsExpressionObjectInputSchemaSchema.nullable().meta({ omitempty: true }).optional(),
  max_branch_width: zod.z.number().int().min(0).max(18446744073709552e3),
  max_leaf_width: zod.z.number().int().min(0).max(18446744073709552e3),
  min_branch_width: zod.z.number().int().min(0).max(18446744073709552e3),
  min_leaf_width: zod.z.number().int().min(0).max(18446744073709552e3),
  name: zod.z.string(),
  readme: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  spec: zod.z.string(),
  tasks: zod.z.array(FunctionsAlphaScalarBranchTaskExpressionSchema).nullable().meta({ omitempty: true }).optional(),
  tasks_length: zod.z.number().int().min(0).max(18446744073709552e3).nullable().meta({ omitempty: true }).optional()
}).meta({ title: "functions.inventions.state.AlphaScalarBranchState" });
var FunctionsInventionsStateAlphaScalarLeafStateSchema = zod.z.object({
  depth: zod.z.number().int().min(0).max(18446744073709552e3),
  description: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  essay: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  essay_tasks: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  input_schema: FunctionsExpressionObjectInputSchemaSchema.nullable().meta({ omitempty: true }).optional(),
  max_branch_width: zod.z.number().int().min(0).max(18446744073709552e3),
  max_leaf_width: zod.z.number().int().min(0).max(18446744073709552e3),
  min_branch_width: zod.z.number().int().min(0).max(18446744073709552e3),
  min_leaf_width: zod.z.number().int().min(0).max(18446744073709552e3),
  name: zod.z.string(),
  readme: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  spec: zod.z.string(),
  tasks: zod.z.array(FunctionsAlphaScalarLeafTaskExpressionSchema).nullable().meta({ omitempty: true }).optional(),
  tasks_length: zod.z.number().int().min(0).max(18446744073709552e3).nullable().meta({ omitempty: true }).optional()
}).meta({ title: "functions.inventions.state.AlphaScalarLeafState" });
var FunctionsInventionsStateAlphaScalarStateSchema = zod.z.object({
  depth: zod.z.number().int().min(0).max(18446744073709552e3),
  input_schema: FunctionsExpressionObjectInputSchemaSchema.nullable().meta({ omitempty: true }).optional(),
  max_branch_width: zod.z.number().int().min(0).max(18446744073709552e3),
  max_leaf_width: zod.z.number().int().min(0).max(18446744073709552e3),
  min_branch_width: zod.z.number().int().min(0).max(18446744073709552e3),
  min_leaf_width: zod.z.number().int().min(0).max(18446744073709552e3),
  name: zod.z.string(),
  spec: zod.z.string()
}).meta({ title: "functions.inventions.state.AlphaScalarState" });
var FunctionsInventionsStateAlphaVectorBranchStateSchema = zod.z.object({
  depth: zod.z.number().int().min(0).max(18446744073709552e3),
  description: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  essay: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  essay_tasks: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  input_schema: FunctionsAlphaVectorExpressionVectorFunctionInputSchemaSchema.nullable().meta({ omitempty: true }).optional(),
  max_branch_width: zod.z.number().int().min(0).max(18446744073709552e3),
  max_leaf_width: zod.z.number().int().min(0).max(18446744073709552e3),
  min_branch_width: zod.z.number().int().min(0).max(18446744073709552e3),
  min_leaf_width: zod.z.number().int().min(0).max(18446744073709552e3),
  name: zod.z.string(),
  readme: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  spec: zod.z.string(),
  tasks: zod.z.array(FunctionsAlphaVectorBranchTaskExpressionSchema).nullable().meta({ omitempty: true }).optional(),
  tasks_length: zod.z.number().int().min(0).max(18446744073709552e3).nullable().meta({ omitempty: true }).optional()
}).meta({ title: "functions.inventions.state.AlphaVectorBranchState" });
var FunctionsInventionsStateAlphaVectorLeafStateSchema = zod.z.object({
  depth: zod.z.number().int().min(0).max(18446744073709552e3),
  description: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  essay: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  essay_tasks: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  input_schema: FunctionsAlphaVectorExpressionVectorFunctionInputSchemaSchema.nullable().meta({ omitempty: true }).optional(),
  max_branch_width: zod.z.number().int().min(0).max(18446744073709552e3),
  max_leaf_width: zod.z.number().int().min(0).max(18446744073709552e3),
  min_branch_width: zod.z.number().int().min(0).max(18446744073709552e3),
  min_leaf_width: zod.z.number().int().min(0).max(18446744073709552e3),
  name: zod.z.string(),
  readme: zod.z.string().nullable().meta({ omitempty: true }).optional(),
  spec: zod.z.string(),
  tasks: zod.z.array(FunctionsAlphaVectorLeafTaskExpressionSchema).nullable().meta({ omitempty: true }).optional(),
  tasks_length: zod.z.number().int().min(0).max(18446744073709552e3).nullable().meta({ omitempty: true }).optional()
}).meta({ title: "functions.inventions.state.AlphaVectorLeafState" });
var FunctionsInventionsStateAlphaVectorStateSchema = zod.z.object({
  depth: zod.z.number().int().min(0).max(18446744073709552e3),
  input_schema: FunctionsAlphaVectorExpressionVectorFunctionInputSchemaSchema.nullable().meta({ omitempty: true }).optional(),
  max_branch_width: zod.z.number().int().min(0).max(18446744073709552e3),
  max_leaf_width: zod.z.number().int().min(0).max(18446744073709552e3),
  min_branch_width: zod.z.number().int().min(0).max(18446744073709552e3),
  min_leaf_width: zod.z.number().int().min(0).max(18446744073709552e3),
  name: zod.z.string(),
  spec: zod.z.string()
}).meta({ title: "functions.inventions.state.AlphaVectorState" });

// src/functions/inventions/state/paramsState.ts
var FunctionsInventionsStateParamsStateSchema = zod.z.union([FunctionsInventionsStateAlphaScalarBranchStateSchema.and(zod.z.object({
  type: zod.z.literal("alpha.scalar.branch.function")
})).meta({ "variantTitle": "AlphaScalarBranch" }), FunctionsInventionsStateAlphaScalarLeafStateSchema.and(zod.z.object({
  type: zod.z.literal("alpha.scalar.leaf.function")
})).meta({ "variantTitle": "AlphaScalarLeaf" }), FunctionsInventionsStateAlphaVectorBranchStateSchema.and(zod.z.object({
  type: zod.z.literal("alpha.vector.branch.function")
})).meta({ "variantTitle": "AlphaVectorBranch" }), FunctionsInventionsStateAlphaVectorLeafStateSchema.and(zod.z.object({
  type: zod.z.literal("alpha.vector.leaf.function")
})).meta({ "variantTitle": "AlphaVectorLeaf" }), FunctionsInventionsStateAlphaScalarStateSchema.and(zod.z.object({
  type: zod.z.literal("alpha.scalar.function")
})).meta({ "variantTitle": "AlphaScalar" }), FunctionsInventionsStateAlphaVectorStateSchema.and(zod.z.object({
  type: zod.z.literal("alpha.vector.function")
})).meta({ "variantTitle": "AlphaVector" })]).meta({ title: "functions.inventions.state.ParamsState" });

// src/functions/inventions/state/paramsStateOrRemoteCommitOptional.ts
var FunctionsInventionsStateParamsStateOrRemoteCommitOptionalSchema = zod.z.union([FunctionsInventionsStateParamsStateSchema.meta({ "title": "functions.inventions.state.ParamsState", "variantTitle": "ParamsState" }), RemotePathCommitOptionalSchema.meta({ "title": "RemotePathCommitOptional", "variantTitle": "Remote" })]).describe("A state specification that is either an inline ParamsState definition\nor a remote path reference.").meta({ title: "functions.inventions.state.ParamsStateOrRemoteCommitOptional" });

// src/functions/inventions/request/functionInventionCreateParams.ts
var FunctionsInventionsRequestFunctionInventionCreateParamsSchema = zod.z.object({
  agent: AgentInlineAgentBaseWithFallbacksOrRemoteCommitOptionalSchema,
  continuation: zod.z.string().nullable().describe("Continuation from a previous completion, as a base64-encoded string.").meta({ omitempty: true }).optional(),
  max_step_retries: zod.z.number().int().min(0).max(4294967295).nullable().describe("Maximum number of retries per invention step.\nEach step is one agent completion (which itself may loop internally\nvia tool calls). If the step's validation still fails after the\nagent loop ends, the step is retried up to this many times.\nDefaults to 3 if not specified.").meta({ omitempty: true }).optional(),
  overwrite: zod.z.boolean().nullable().meta({ omitempty: true }).optional(),
  prompt: FunctionsInventionsPromptsInlinePromptOrRemoteCommitOptionalSchema,
  provider: AgentCompletionsRequestProviderSchema.nullable().meta({ omitempty: true }).optional(),
  remote: RemoteSchema.nullable().meta({ omitempty: true }).optional(),
  seed: zod.z.number().int().min(-9223372036854776e3).max(9223372036854776e3).nullable().meta({ omitempty: true }).optional(),
  state: FunctionsInventionsStateParamsStateOrRemoteCommitOptionalSchema,
  stream: zod.z.boolean().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "functions.inventions.request.FunctionInventionCreateParams" });
var FunctionsAlphaScalarRemoteFunctionSchema = zod.z.union([zod.z.object({
  description: zod.z.string(),
  input_schema: FunctionsExpressionObjectInputSchemaSchema,
  tasks: zod.z.array(FunctionsAlphaScalarBranchTaskExpressionSchema),
  type: zod.z.literal("alpha.scalar.branch.function")
}).meta({ "variantTitle": "Branch" }), zod.z.object({
  description: zod.z.string(),
  input_schema: FunctionsExpressionObjectInputSchemaSchema,
  tasks: zod.z.array(FunctionsAlphaScalarLeafTaskExpressionSchema),
  type: zod.z.literal("alpha.scalar.leaf.function")
}).meta({ "variantTitle": "Leaf" })]).meta({ title: "functions.alpha_scalar.RemoteFunction" });
var FunctionsAlphaVectorRemoteFunctionSchema = zod.z.union([zod.z.object({
  description: zod.z.string(),
  input_schema: FunctionsAlphaVectorExpressionVectorFunctionInputSchemaSchema,
  tasks: zod.z.array(FunctionsAlphaVectorBranchTaskExpressionSchema),
  type: zod.z.literal("alpha.vector.branch.function")
}).meta({ "variantTitle": "Branch" }), zod.z.object({
  description: zod.z.string(),
  input_schema: FunctionsAlphaVectorExpressionVectorFunctionInputSchemaSchema,
  tasks: zod.z.array(FunctionsAlphaVectorLeafTaskExpressionSchema),
  type: zod.z.literal("alpha.vector.leaf.function")
}).meta({ "variantTitle": "Leaf" })]).meta({ title: "functions.alpha_vector.RemoteFunction" });

// src/functions/alphaRemoteFunction.ts
var FunctionsAlphaRemoteFunctionSchema = zod.z.union([FunctionsAlphaScalarRemoteFunctionSchema.meta({ "title": "functions.alpha_scalar.RemoteFunction", "variantTitle": "Scalar" }), FunctionsAlphaVectorRemoteFunctionSchema.meta({ "title": "functions.alpha_vector.RemoteFunction", "variantTitle": "Vector" })]).meta({ title: "functions.AlphaRemoteFunction" });
var FunctionsRemoteFunctionSchema = zod.z.union([zod.z.object({
  description: zod.z.string().describe("Human-readable description of what the function does."),
  input_schema: FunctionsExpressionInputSchemaSchema.describe("JSON Schema defining the expected input structure."),
  tasks: zod.z.array(FunctionsTaskExpressionSchema).describe("The list of tasks to execute. Tasks with a `map` expression are\nexpanded into multiple instances. Each instance is compiled with\n`map` set to the current integer index.\nReceives: `input`, `map` (if mapped)."),
  type: zod.z.literal("scalar.function")
}).describe("Produces a single score in [0, 1].").meta({ "variantTitle": "Scalar" }), zod.z.object({
  description: zod.z.string().describe("Human-readable description of what the function does."),
  input_merge: FunctionsExpressionExpressionSchema.describe("Expression transforming an array of inputs computed by `input_split`\ninto a single Input object for the Function.\nReceives: `input` (as an array)."),
  input_schema: FunctionsExpressionInputSchemaSchema.describe("JSON Schema defining the expected input structure."),
  input_split: FunctionsExpressionExpressionSchema.describe("Expression transforming input into an input array of the output_length\nWhen the Function is executed with any input from the array,\nThe output_length should be 1.\nReceives: `input`."),
  output_length: FunctionsExpressionExpressionSchema.describe("Expression computing the expected output vector length for task outputs.\nReceives: `input`."),
  tasks: zod.z.array(FunctionsTaskExpressionSchema).describe("The list of tasks to execute. Tasks with a `map` expression are\nexpanded into multiple instances. Each instance is compiled with\n`map` set to the current integer index.\nReceives: `input`, `map` (if mapped)."),
  type: zod.z.literal("vector.function")
}).describe("Produces a vector of scores that sums to 1.").meta({ "variantTitle": "Vector" })]).describe("A remote function with full metadata.\n\nRemote functions are stored as `function.json` in repositories and\nreferenced by `remote/owner/repository`. They include documentation fields\nthat inline functions lack.").meta({ title: "functions.RemoteFunction" });

// src/functions/fullRemoteFunction.ts
var FunctionsFullRemoteFunctionSchema = zod.z.union([FunctionsAlphaRemoteFunctionSchema.meta({ "title": "functions.AlphaRemoteFunction", "variantTitle": "Alpha" }), FunctionsRemoteFunctionSchema.meta({ "title": "functions.RemoteFunction", "variantTitle": "Standard" })]).meta({ title: "functions.FullRemoteFunction" });
var FunctionsInventionsResponseStreamingObjectSchema = zod.z.enum(["alpha.scalar.function.invention.chunk", "alpha.vector.function.invention.chunk"]).meta({ title: "functions.inventions.response.streaming.Object" });
var FunctionsInventionsStateStateSchema = zod.z.union([FunctionsInventionsStateAlphaScalarBranchStateSchema.and(zod.z.object({
  type: zod.z.literal("alpha.scalar.branch.function")
})).meta({ "variantTitle": "AlphaScalarBranch" }), FunctionsInventionsStateAlphaScalarLeafStateSchema.and(zod.z.object({
  type: zod.z.literal("alpha.scalar.leaf.function")
})).meta({ "variantTitle": "AlphaScalarLeaf" }), FunctionsInventionsStateAlphaVectorBranchStateSchema.and(zod.z.object({
  type: zod.z.literal("alpha.vector.branch.function")
})).meta({ "variantTitle": "AlphaVectorBranch" }), FunctionsInventionsStateAlphaVectorLeafStateSchema.and(zod.z.object({
  type: zod.z.literal("alpha.vector.leaf.function")
})).meta({ "variantTitle": "AlphaVectorLeaf" })]).meta({ title: "functions.inventions.state.State" });

// src/functions/inventions/response/streaming/functionInventionChunkLog.ts
var FunctionsInventionsResponseStreamingFunctionInventionChunkLogSchema = zod.z.object({
  completions: zod.z.array(IndexedLogReferenceSchema),
  created: zod.z.number().int().min(0).max(18446744073709552e3),
  error: ErrorResponseErrorSchema.nullable().meta({ omitempty: true }).optional(),
  function: FunctionsFullRemoteFunctionSchema.nullable().meta({ omitempty: true }).optional(),
  id: zod.z.string(),
  object: FunctionsInventionsResponseStreamingObjectSchema,
  path: RemotePathSchema.nullable().meta({ omitempty: true }).optional(),
  state: FunctionsInventionsStateStateSchema.nullable().meta({ omitempty: true }).optional(),
  usage: AgentCompletionsResponseUsageSchema.nullable().meta({ omitempty: true }).optional()
}).meta({ title: "functions.inventions.response.streaming.FunctionInventionChunkLog" });
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
var VectorCompletionsResponseStreamingObjectSchema = zod.z.literal("vector.completion.chunk").describe("A streaming vector completion chunk.").meta({ title: "vector.completions.response.streaming.Object" });
var VectorCompletionsResponseVoteSchema = zod.z.object({
  agent: zod.z.string().describe("The agent that produced this vote (content-addressed ID)."),
  flat_swarm_index: zod.z.number().int().min(0).max(18446744073709552e3).describe("Flattened index accounting for agent counts in the swarm."),
  from_cache: zod.z.boolean().nullable().describe("If true, this vote was retrieved from cache rather than generated fresh.").meta({ omitempty: true }).optional(),
  prompt_id: zod.z.string().describe("Content hash of the request messages (for caching/deduplication)."),
  responses_ids: zod.z.array(zod.z.string()).describe("Content hashes of each response option in the request."),
  retry: zod.z.boolean().nullable().describe("If true, this vote was reused from a previous request via the `retry`\nparameter. All fields reflect the original request's values.").meta({ omitempty: true }).optional(),
  swarm_index: zod.z.number().int().min(0).max(18446744073709552e3).describe("Index of the agent configuration within the swarm."),
  vote: zod.z.array(zod.z.number().min(-34028234663852886e22).max(34028234663852886e22)).describe("The vote distribution. Each index corresponds to a response from the\nrequest. Typically one element is 1.0 (selected) and the rest are 0.0."),
  weight: zod.z.number().min(-34028234663852886e22).max(34028234663852886e22).describe("The weight applied to this vote when computing final scores.")
}).describe("A single LLM's vote in a vector completion.\n\nEach LLM in the swarm produces a vote indicating which response(s) it\nselected. Votes are weighted according to the profile and combined to\nproduce the final scores.\n\n# Vote Format\n\nThe `vote` field is a vector of decimals corresponding to the responses\nin the request. Typically one element is 1.0 and the rest are 0.0 (discrete\nselection), but when `top_logprobs` is used, votes may be probability\ndistributions.").meta({ title: "vector.completions.response.Vote" });

// src/vector/completions/response/streaming/vectorCompletionChunkLog.ts
var VectorCompletionsResponseStreamingVectorCompletionChunkLogSchema = zod.z.object({
  completions: zod.z.array(IndexedLogReferenceSchema),
  created: zod.z.number().int().min(0).max(18446744073709552e3),
  id: zod.z.string(),
  object: VectorCompletionsResponseStreamingObjectSchema,
  scores: zod.z.array(zod.z.number().min(-34028234663852886e22).max(34028234663852886e22)),
  swarm: zod.z.string(),
  usage: AgentCompletionsResponseUsageSchema.nullable().meta({ omitempty: true }).optional(),
  votes: zod.z.array(VectorCompletionsResponseVoteSchema),
  weights: zod.z.array(zod.z.number().min(-34028234663852886e22).max(34028234663852886e22))
}).meta({ title: "vector.completions.response.streaming.VectorCompletionChunkLog" });

// src/cli/command/agents/read/id/response.ts
var CliCommandAgentsReadIdResponseSchema = zod.z.union([AgentCompletionsResponseStreamingAgentCompletionChunkLogSchema.meta({ "title": "agent.completions.response.streaming.AgentCompletionChunkLog", "variantTitle": "AgentsCompletionsResponse" }), AgentCompletionsRequestAgentCompletionCreateParamsLogSchema.meta({ "title": "agent.completions.request.AgentCompletionCreateParamsLog", "variantTitle": "AgentsCompletionsRequest" }), AgentCompletionsMessageMessageLogSchema.meta({ "title": "agent.completions.message.MessageLog", "variantTitle": "AgentsCompletionsResponseMessages" }), AgentCompletionsResponseLogprobsSchema.meta({ "title": "agent.completions.response.Logprobs", "variantTitle": "AgentsCompletionsResponseMessagesLogprobs" }), AgentCompletionsMessageAssistantToolCallDeltaSchema.meta({ "title": "agent.completions.message.AssistantToolCallDelta", "variantTitle": "AgentsCompletionsResponseMessagesToolCalls" }), VectorCompletionsResponseStreamingVectorCompletionChunkLogSchema.meta({ "title": "vector.completions.response.streaming.VectorCompletionChunkLog", "variantTitle": "VectorCompletionsResponse" }), VectorCompletionsRequestVectorCompletionCreateParamsSchema.meta({ "title": "vector.completions.request.VectorCompletionCreateParams", "variantTitle": "VectorCompletionsRequest" }), FunctionsExecutionsResponseStreamingFunctionExecutionChunkLogSchema.meta({ "title": "functions.executions.response.streaming.FunctionExecutionChunkLog", "variantTitle": "FunctionsExecutionsResponse" }), FunctionsExecutionsRequestFunctionExecutionCreateParamsLogSchema.meta({ "title": "functions.executions.request.FunctionExecutionCreateParamsLog", "variantTitle": "FunctionsExecutionsRequest" }), FunctionsInventionsResponseStreamingFunctionInventionChunkLogSchema.meta({ "title": "functions.inventions.response.streaming.FunctionInventionChunkLog", "variantTitle": "FunctionsInventionsResponse" }), FunctionsInventionsRequestFunctionInventionCreateParamsSchema.meta({ "title": "functions.inventions.request.FunctionInventionCreateParams", "variantTitle": "FunctionsInventionsRequest" }), FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunkLogSchema.meta({ "title": "functions.inventions.recursive.response.streaming.FunctionInventionRecursiveChunkLog", "variantTitle": "FunctionsInventionsRecursiveResponse" }), FunctionsInventionsRecursiveRequestFunctionInventionRecursiveCreateParamsLogSchema.meta({ "title": "functions.inventions.recursive.request.FunctionInventionRecursiveCreateParamsLog", "variantTitle": "FunctionsInventionsRecursiveRequest" }), zod.z.string().meta({ "variantTitle": "Text" }), AgentCompletionsMessageImageUrlSchema.meta({ "title": "agent.completions.message.ImageUrl", "variantTitle": "Image" }), AgentCompletionsMessageInputAudioSchema.meta({ "title": "agent.completions.message.InputAudio", "variantTitle": "Audio" }), AgentCompletionsMessageVideoUrlSchema.meta({ "title": "agent.completions.message.VideoUrl", "variantTitle": "Video" }), AgentCompletionsMessageFileSchema.meta({ "title": "agent.completions.message.File", "variantTitle": "File" })]).meta({ title: "cli.command.agents.read.id.Response" });

// src/viewer/command/agents/read/id.ts
async function agentsReadIdExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/read/id" }), zod.z.union([CliErrorSchema, CliCommandAgentsReadIdResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents read id: cli produced no output before the end marker");
  }
  return first;
}
async function agentsReadIdExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/read/id" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents read id: cli produced no output before the end marker");
  }
  return first;
}
async function agentsReadIdRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/read/id/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents read id request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsReadIdRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/read/id/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents read id request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsReadIdResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/read/id/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents read id response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsReadIdResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/read/id/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents read id response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandAgentsReadPendingResponseItemSchema = zod.z.object({
  agent_id: zod.z.string(),
  items: zod.z.array(CliCommandAgentsReadAllResponseQueueItemSchema)
}).meta({ title: "cli.command.agents.read.pending.ResponseItem" });

// src/viewer/command/agents/read/pending.ts
function agentsReadPendingExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/read/pending" }), zod.z.union([CliErrorSchema, CliCommandAgentsReadPendingResponseItemSchema]));
}
function agentsReadPendingExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/read/pending" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function agentsReadPendingRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/read/pending/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents read pending request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsReadPendingRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/read/pending/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents read pending request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsReadPendingResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/read/pending/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents read pending response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsReadPendingResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/read/pending/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents read pending response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandAgentsReadSubscribeResponseItemSchema = zod.z.union([zod.z.object({
  agent_id: zod.z.string(),
  items: zod.z.array(CliCommandAgentsReadAllResponseQueueItemSchema)
}).meta({ "variantTitle": "Items" }), zod.z.object({
  agent_id: zod.z.string()
}).meta({ "variantTitle": "Inactive" })]).meta({ title: "cli.command.agents.read.subscribe.ResponseItem" });

// src/viewer/command/agents/read/subscribe.ts
function agentsReadSubscribeExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/read/subscribe" }), zod.z.union([CliErrorSchema, CliCommandAgentsReadSubscribeResponseItemSchema]));
}
function agentsReadSubscribeExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/read/subscribe" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function agentsReadSubscribeRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/read/subscribe/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents read subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsReadSubscribeRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/read/subscribe/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents read subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsReadSubscribeResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/read/subscribe/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents read subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsReadSubscribeResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/read/subscribe/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents read subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
var AgentCompletionsResponseAssistantRoleSchema = zod.z.literal("assistant").describe("The assistant role.").meta({ title: "agent.completions.response.AssistantRole" });
var AgentCompletionsResponseFinishReasonSchema = zod.z.union([zod.z.literal("stop").describe("The model reached a natural stop point or stop sequence.").meta({ "variantTitle": "Stop" }), zod.z.literal("length").describe("The model reached the maximum token limit.").meta({ "variantTitle": "Length" }), zod.z.literal("tool_calls").describe("The model decided to call one or more tools.").meta({ "variantTitle": "ToolCalls" }), zod.z.literal("content_filter").describe("The response was filtered due to content policy.").meta({ "variantTitle": "ContentFilter" }), zod.z.literal("error").describe("An error occurred during generation.").meta({ "variantTitle": "Error" })]).describe("The reason the model stopped generating.").meta({ title: "agent.completions.response.FinishReason" });
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
  role: AgentCompletionsResponseToolRoleSchema,
  tool_call_id: zod.z.string().describe("The ID of the tool call this message responds to.")
}).describe("A tool message containing the result of a tool call.").meta({ title: "agent.completions.response.ToolResponse" });

// src/agent/completions/response/streaming/messageChunk.ts
var AgentCompletionsResponseStreamingMessageChunkSchema = zod.z.union([AgentCompletionsResponseStreamingAssistantResponseChunkSchema.meta({ "title": "agent.completions.response.streaming.AssistantResponseChunk", "variantTitle": "Assistant" }), AgentCompletionsResponseToolResponseSchema.meta({ "title": "agent.completions.response.ToolResponse", "variantTitle": "Tool" })]).meta({ title: "agent.completions.response.streaming.MessageChunk" });

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
var CliCommandConfigFunctionsInventionsGetResponseSchema = zod.z.object({
  remote: RemoteSchema
}).meta({ title: "cli.command.config.functions.inventions.get.Response" });

// src/viewer/command/config/functions/inventions/get.ts
async function configFunctionsInventionsGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/inventions/get" }), zod.z.union([CliErrorSchema, CliCommandConfigFunctionsInventionsGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions inventions get: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsInventionsGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/inventions/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions inventions get: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsInventionsGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/inventions/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions inventions get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsInventionsGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/inventions/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions inventions get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsInventionsGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/inventions/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions inventions get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsInventionsGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/inventions/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions inventions get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsInventionsRemoteGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/inventions/remote/get" }), zod.z.union([CliErrorSchema, RemoteSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions inventions remote get: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsInventionsRemoteGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/inventions/remote/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions inventions remote get: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsInventionsRemoteGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/inventions/remote/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions inventions remote get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsInventionsRemoteGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/inventions/remote/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions inventions remote get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsInventionsRemoteGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/inventions/remote/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions inventions remote get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsInventionsRemoteGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/inventions/remote/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions inventions remote get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsInventionsRemoteSetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/functions/inventions/remote/set" }), zod.z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions inventions remote set: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsInventionsRemoteSetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/functions/inventions/remote/set" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions inventions remote set: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsInventionsRemoteSetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/inventions/remote/set/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions inventions remote set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsInventionsRemoteSetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/inventions/remote/set/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions inventions remote set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsInventionsRemoteSetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/inventions/remote/set/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions inventions remote set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsInventionsRemoteSetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/inventions/remote/set/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions inventions remote set response_schema: cli produced no output before the end marker");
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

// src/cli/command/functions/executions/create/standard/responseItem.ts
var CliCommandFunctionsExecutionsCreateStandardResponseItemSchema = zod.z.union([FunctionsExecutionsResponseStreamingFunctionExecutionChunkSchema.meta({ "title": "functions.executions.response.streaming.FunctionExecutionChunk", "variantTitle": "Chunk" }), zod.z.string().meta({ "variantTitle": "Id" })]).meta({ title: "cli.command.functions.executions.create.standard.ResponseItem" });

// src/viewer/command/functions/executions/create/standard.ts
function functionsExecutionsCreateStandardExecuteStreaming(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, dangerous_advanced: { ...request.dangerous_advanced ?? {}, stream: true }, path_type: "functions/executions/create/standard" }), zod.z.union([CliErrorSchema, CliCommandFunctionsExecutionsCreateStandardResponseItemSchema]));
}
function functionsExecutionsCreateStandardExecuteStreamingJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, dangerous_advanced: { ...request.dangerous_advanced ?? {}, stream: true }, path_type: "functions/executions/create/standard" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function functionsExecutionsCreateStandardExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, dangerous_advanced: request.dangerous_advanced ? { ...request.dangerous_advanced, stream: void 0 } : request.dangerous_advanced, path_type: "functions/executions/create/standard" }), zod.z.union([CliErrorSchema, zod.z.string()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions executions create standard: cli produced no output before the end marker");
  }
  return first;
}
async function functionsExecutionsCreateStandardExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, dangerous_advanced: request.dangerous_advanced ? { ...request.dangerous_advanced, stream: void 0 } : request.dangerous_advanced, path_type: "functions/executions/create/standard" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions executions create standard: cli produced no output before the end marker");
  }
  return first;
}
async function functionsExecutionsCreateStandardRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/executions/create/standard/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions executions create standard request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsExecutionsCreateStandardRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/executions/create/standard/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions executions create standard request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsExecutionsCreateStandardResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/executions/create/standard/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions executions create standard response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsExecutionsCreateStandardResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/executions/create/standard/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions executions create standard response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandFunctionsExecutionsCreateSwissSystemResponseItemSchema = zod.z.union([FunctionsExecutionsResponseStreamingFunctionExecutionChunkSchema.meta({ "title": "functions.executions.response.streaming.FunctionExecutionChunk", "variantTitle": "Chunk" }), zod.z.string().meta({ "variantTitle": "Id" })]).meta({ title: "cli.command.functions.executions.create.swiss_system.ResponseItem" });

// src/viewer/command/functions/executions/create/swiss_system.ts
function functionsExecutionsCreateSwissSystemExecuteStreaming(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, dangerous_advanced: { ...request.dangerous_advanced ?? {}, stream: true }, path_type: "functions/executions/create/swiss_system" }), zod.z.union([CliErrorSchema, CliCommandFunctionsExecutionsCreateSwissSystemResponseItemSchema]));
}
function functionsExecutionsCreateSwissSystemExecuteStreamingJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, dangerous_advanced: { ...request.dangerous_advanced ?? {}, stream: true }, path_type: "functions/executions/create/swiss_system" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function functionsExecutionsCreateSwissSystemExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, dangerous_advanced: request.dangerous_advanced ? { ...request.dangerous_advanced, stream: void 0 } : request.dangerous_advanced, path_type: "functions/executions/create/swiss_system" }), zod.z.union([CliErrorSchema, zod.z.string()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions executions create swiss_system: cli produced no output before the end marker");
  }
  return first;
}
async function functionsExecutionsCreateSwissSystemExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, dangerous_advanced: request.dangerous_advanced ? { ...request.dangerous_advanced, stream: void 0 } : request.dangerous_advanced, path_type: "functions/executions/create/swiss_system" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions executions create swiss_system: cli produced no output before the end marker");
  }
  return first;
}
async function functionsExecutionsCreateSwissSystemRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/executions/create/swiss_system/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions executions create swiss_system request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsExecutionsCreateSwissSystemRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/executions/create/swiss_system/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions executions create swiss_system request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsExecutionsCreateSwissSystemResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/executions/create/swiss_system/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions executions create swiss_system response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsExecutionsCreateSwissSystemResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/executions/create/swiss_system/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions executions create swiss_system response_schema: cli produced no output before the end marker");
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
var FunctionsInventionsResponseStreamingAgentCompletionChunkSchema = zod.z.object({
  agent_full_id: zod.z.string().describe("WF-level id: concatenation of the primary agent's id with all\nfallback ids (see `InlineAgentWithFallbacks::full_id`). Same\nfor every slot in the same WF request."),
  agent_id: zod.z.string().describe("Leaf agent id of the slot that produced this chunk. For the\nprimary attempt this is the primary agent's id; on fallback it\nis the fallback agent's id. Same on every chunk of a slot."),
  agent_instance_hierarchy: zod.z.string().describe("Full agent instance hierarchy for this completion's slot \u2014\n`{ctx lineage}/{agent_full_id}-{response_id}`, or the fixed\ncontinuation value on resume. Same on every chunk of a slot."),
  agent_remote: RemotePathSchema.nullable().describe("`RemotePath` the WF was fetched from. `None` when the WF was\nsupplied inline. Same for every slot in the same WF request.").meta({ omitempty: true }).optional(),
  continuation: zod.z.string().nullable().describe("Continuation state for multi-turn conversations (only present in the final chunk).").meta({ omitempty: true }).optional(),
  created: zod.z.number().int().min(0).max(18446744073709552e3),
  error: ErrorResponseErrorSchema.nullable().describe("Error details if this completion failed.").meta({ omitempty: true }).optional(),
  id: zod.z.string(),
  index: zod.z.number().int().min(0).max(18446744073709552e3),
  messages: zod.z.array(AgentCompletionsResponseStreamingMessageChunkSchema),
  messages_queued: zod.z.boolean().nullable().describe("`true` when the MCP proxy holds queued messages that were not\ndelivered to the agent via a tool response on this turn. Only\nset when `continuation` is also set \u2014 the caller acts on it by\nissuing the continuation. Absent when nothing is queued, when\nthere is no continuation to act on, or when the peek failed\n(the failure is surfaced via `error`).").meta({ omitempty: true }).optional(),
  object: AgentCompletionsResponseStreamingObjectSchema.describe('The object type (always "agent.completion.chunk").'),
  upstream: AgentUpstreamSchema.describe("Upstream provider"),
  usage: AgentCompletionsResponseUsageSchema.nullable().describe("Token usage (only present in the final chunk).").meta({ omitempty: true }).optional()
}).describe("A chunk of a streaming agent completion response.\n\nMultiple chunks are received via Server-Sent Events and can be\naccumulated into a complete [`AgentCompletion`](response::unary::AgentCompletion)\nusing the [`push`](Self::push) method.").meta({ title: "functions.inventions.response.streaming.AgentCompletionChunk" });

// src/functions/inventions/recursive/response/streaming/functionInventionChunk.ts
var FunctionsInventionsRecursiveResponseStreamingFunctionInventionChunkSchema = zod.z.object({
  completions: zod.z.array(FunctionsInventionsResponseStreamingAgentCompletionChunkSchema),
  created: zod.z.number().int().min(0).max(18446744073709552e3),
  error: ErrorResponseErrorSchema.nullable().meta({ omitempty: true }).optional(),
  function: FunctionsFullRemoteFunctionSchema.nullable().meta({ omitempty: true }).optional(),
  id: zod.z.string(),
  index: zod.z.number().int().min(0).max(18446744073709552e3),
  object: FunctionsInventionsResponseStreamingObjectSchema,
  path: RemotePathSchema.nullable().meta({ omitempty: true }).optional(),
  state: FunctionsInventionsStateStateSchema.nullable().meta({ omitempty: true }).optional(),
  usage: AgentCompletionsResponseUsageSchema.nullable().meta({ omitempty: true }).optional()
}).meta({ title: "functions.inventions.recursive.response.streaming.FunctionInventionChunk" });

// src/functions/inventions/recursive/response/streaming/functionInventionRecursiveChunk.ts
var FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunkSchema = zod.z.object({
  created: zod.z.number().int().min(0).max(18446744073709552e3),
  id: zod.z.string(),
  inventions: zod.z.array(FunctionsInventionsRecursiveResponseStreamingFunctionInventionChunkSchema),
  inventions_errors: zod.z.boolean().nullable().meta({ omitempty: true }).optional(),
  object: FunctionsInventionsRecursiveResponseStreamingObjectSchema,
  usage: AgentCompletionsResponseUsageSchema.nullable().meta({ omitempty: true }).optional()
}).meta({ title: "functions.inventions.recursive.response.streaming.FunctionInventionRecursiveChunk" });

// src/cli/command/functions/inventions/recursive/create/alpha_scalar/responseItem.ts
var CliCommandFunctionsInventionsRecursiveCreateAlphaScalarResponseItemSchema = zod.z.union([FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunkSchema.meta({ "title": "functions.inventions.recursive.response.streaming.FunctionInventionRecursiveChunk", "variantTitle": "Chunk" }), zod.z.string().meta({ "variantTitle": "Id" })]).meta({ title: "cli.command.functions.inventions.recursive.create.alpha_scalar.ResponseItem" });

// src/viewer/command/functions/inventions/recursive/create/alpha_scalar.ts
function functionsInventionsRecursiveCreateAlphaScalarExecuteStreaming(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, dangerous_advanced: { ...request.dangerous_advanced ?? {}, stream: true }, path_type: "functions/inventions/recursive/create/alpha_scalar" }), zod.z.union([CliErrorSchema, CliCommandFunctionsInventionsRecursiveCreateAlphaScalarResponseItemSchema]));
}
function functionsInventionsRecursiveCreateAlphaScalarExecuteStreamingJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, dangerous_advanced: { ...request.dangerous_advanced ?? {}, stream: true }, path_type: "functions/inventions/recursive/create/alpha_scalar" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function functionsInventionsRecursiveCreateAlphaScalarExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, dangerous_advanced: request.dangerous_advanced ? { ...request.dangerous_advanced, stream: void 0 } : request.dangerous_advanced, path_type: "functions/inventions/recursive/create/alpha_scalar" }), zod.z.union([CliErrorSchema, zod.z.string()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions inventions recursive create alpha_scalar: cli produced no output before the end marker");
  }
  return first;
}
async function functionsInventionsRecursiveCreateAlphaScalarExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, dangerous_advanced: request.dangerous_advanced ? { ...request.dangerous_advanced, stream: void 0 } : request.dangerous_advanced, path_type: "functions/inventions/recursive/create/alpha_scalar" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions inventions recursive create alpha_scalar: cli produced no output before the end marker");
  }
  return first;
}
async function functionsInventionsRecursiveCreateAlphaScalarRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/inventions/recursive/create/alpha_scalar/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions inventions recursive create alpha_scalar request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsInventionsRecursiveCreateAlphaScalarRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/inventions/recursive/create/alpha_scalar/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions inventions recursive create alpha_scalar request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsInventionsRecursiveCreateAlphaScalarResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/inventions/recursive/create/alpha_scalar/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions inventions recursive create alpha_scalar response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsInventionsRecursiveCreateAlphaScalarResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/inventions/recursive/create/alpha_scalar/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions inventions recursive create alpha_scalar response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandFunctionsInventionsRecursiveCreateAlphaVectorResponseItemSchema = zod.z.union([FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunkSchema.meta({ "title": "functions.inventions.recursive.response.streaming.FunctionInventionRecursiveChunk", "variantTitle": "Chunk" }), zod.z.string().meta({ "variantTitle": "Id" })]).meta({ title: "cli.command.functions.inventions.recursive.create.alpha_vector.ResponseItem" });

// src/viewer/command/functions/inventions/recursive/create/alpha_vector.ts
function functionsInventionsRecursiveCreateAlphaVectorExecuteStreaming(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, dangerous_advanced: { ...request.dangerous_advanced ?? {}, stream: true }, path_type: "functions/inventions/recursive/create/alpha_vector" }), zod.z.union([CliErrorSchema, CliCommandFunctionsInventionsRecursiveCreateAlphaVectorResponseItemSchema]));
}
function functionsInventionsRecursiveCreateAlphaVectorExecuteStreamingJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, dangerous_advanced: { ...request.dangerous_advanced ?? {}, stream: true }, path_type: "functions/inventions/recursive/create/alpha_vector" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function functionsInventionsRecursiveCreateAlphaVectorExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, dangerous_advanced: request.dangerous_advanced ? { ...request.dangerous_advanced, stream: void 0 } : request.dangerous_advanced, path_type: "functions/inventions/recursive/create/alpha_vector" }), zod.z.union([CliErrorSchema, zod.z.string()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions inventions recursive create alpha_vector: cli produced no output before the end marker");
  }
  return first;
}
async function functionsInventionsRecursiveCreateAlphaVectorExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, dangerous_advanced: request.dangerous_advanced ? { ...request.dangerous_advanced, stream: void 0 } : request.dangerous_advanced, path_type: "functions/inventions/recursive/create/alpha_vector" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions inventions recursive create alpha_vector: cli produced no output before the end marker");
  }
  return first;
}
async function functionsInventionsRecursiveCreateAlphaVectorRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/inventions/recursive/create/alpha_vector/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions inventions recursive create alpha_vector request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsInventionsRecursiveCreateAlphaVectorRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/inventions/recursive/create/alpha_vector/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions inventions recursive create alpha_vector request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsInventionsRecursiveCreateAlphaVectorResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/inventions/recursive/create/alpha_vector/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions inventions recursive create alpha_vector response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsInventionsRecursiveCreateAlphaVectorResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/inventions/recursive/create/alpha_vector/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions inventions recursive create alpha_vector response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandFunctionsInventionsRecursiveCreateRemoteResponseItemSchema = zod.z.union([FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunkSchema.meta({ "title": "functions.inventions.recursive.response.streaming.FunctionInventionRecursiveChunk", "variantTitle": "Chunk" }), zod.z.string().meta({ "variantTitle": "Id" })]).meta({ title: "cli.command.functions.inventions.recursive.create.remote.ResponseItem" });

// src/viewer/command/functions/inventions/recursive/create/remote.ts
function functionsInventionsRecursiveCreateRemoteExecuteStreaming(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, dangerous_advanced: { ...request.dangerous_advanced ?? {}, stream: true }, path_type: "functions/inventions/recursive/create/remote" }), zod.z.union([CliErrorSchema, CliCommandFunctionsInventionsRecursiveCreateRemoteResponseItemSchema]));
}
function functionsInventionsRecursiveCreateRemoteExecuteStreamingJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, dangerous_advanced: { ...request.dangerous_advanced ?? {}, stream: true }, path_type: "functions/inventions/recursive/create/remote" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function functionsInventionsRecursiveCreateRemoteExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, dangerous_advanced: request.dangerous_advanced ? { ...request.dangerous_advanced, stream: void 0 } : request.dangerous_advanced, path_type: "functions/inventions/recursive/create/remote" }), zod.z.union([CliErrorSchema, zod.z.string()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions inventions recursive create remote: cli produced no output before the end marker");
  }
  return first;
}
async function functionsInventionsRecursiveCreateRemoteExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, dangerous_advanced: request.dangerous_advanced ? { ...request.dangerous_advanced, stream: void 0 } : request.dangerous_advanced, path_type: "functions/inventions/recursive/create/remote" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions inventions recursive create remote: cli produced no output before the end marker");
  }
  return first;
}
async function functionsInventionsRecursiveCreateRemoteRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/inventions/recursive/create/remote/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions inventions recursive create remote request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsInventionsRecursiveCreateRemoteRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/inventions/recursive/create/remote/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions inventions recursive create remote request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsInventionsRecursiveCreateRemoteResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/inventions/recursive/create/remote/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions inventions recursive create remote response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsInventionsRecursiveCreateRemoteResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/inventions/recursive/create/remote/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions inventions recursive create remote response_schema: cli produced no output before the end marker");
  }
  return first;
}
var FunctionsInventionsStateGetFunctionInventionStateResponseSchema = RemotePathSchema.and(zod.z.object({})).describe("Response from retrieving a function invention state.").meta({ title: "functions.inventions.state.GetFunctionInventionStateResponse" });

// src/viewer/command/functions/inventions/state/get.ts
async function functionsInventionsStateGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/inventions/state/get" }), zod.z.union([CliErrorSchema, FunctionsInventionsStateGetFunctionInventionStateResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions inventions state get: cli produced no output before the end marker");
  }
  return first;
}
async function functionsInventionsStateGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/inventions/state/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions inventions state get: cli produced no output before the end marker");
  }
  return first;
}
async function functionsInventionsStateGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/inventions/state/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions inventions state get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsInventionsStateGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/inventions/state/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions inventions state get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsInventionsStateGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/inventions/state/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions inventions state get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsInventionsStateGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/inventions/state/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions inventions state get response_schema: cli produced no output before the end marker");
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
async function logsAgentsCompletionsRequestGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/get" }), zod.z.union([CliErrorSchema, AgentCompletionsRequestAgentCompletionCreateParamsLogSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesAudioGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/messages/audio/get" }), zod.z.union([CliErrorSchema, AgentCompletionsMessageInputAudioSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages audio get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesAudioGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/messages/audio/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages audio get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesAudioGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/messages/audio/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages audio get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesAudioGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/messages/audio/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages audio get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesAudioGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/messages/audio/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages audio get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesAudioGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/messages/audio/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages audio get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesFileGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/messages/file/get" }), zod.z.union([CliErrorSchema, AgentCompletionsMessageFileSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages file get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesFileGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/messages/file/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages file get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesFileGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/messages/file/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages file get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesFileGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/messages/file/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages file get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesFileGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/messages/file/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages file get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesFileGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/messages/file/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages file get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesImageGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/messages/image/get" }), zod.z.union([CliErrorSchema, AgentCompletionsMessageImageUrlSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages image get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesImageGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/messages/image/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages image get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesImageGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/messages/image/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages image get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesImageGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/messages/image/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages image get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesImageGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/messages/image/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages image get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesImageGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/messages/image/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages image get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesTextGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/messages/text/get" }), zod.z.union([CliErrorSchema, zod.z.string()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages text get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesTextGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/messages/text/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages text get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesTextGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/messages/text/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages text get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesTextGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/messages/text/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages text get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesTextGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/messages/text/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages text get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesTextGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/messages/text/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages text get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesVideoGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/messages/video/get" }), zod.z.union([CliErrorSchema, AgentCompletionsMessageVideoUrlSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages video get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesVideoGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/messages/video/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages video get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesVideoGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/messages/video/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages video get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesVideoGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/messages/video/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages video get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesVideoGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/messages/video/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages video get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesVideoGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/messages/video/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages video get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsAudioGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/notifications/audio/get" }), zod.z.union([CliErrorSchema, AgentCompletionsMessageInputAudioSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications audio get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsAudioGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/notifications/audio/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications audio get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsAudioGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/notifications/audio/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications audio get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsAudioGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/notifications/audio/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications audio get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsAudioGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/notifications/audio/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications audio get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsAudioGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/notifications/audio/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications audio get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsFileGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/notifications/file/get" }), zod.z.union([CliErrorSchema, AgentCompletionsMessageFileSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications file get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsFileGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/notifications/file/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications file get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsFileGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/notifications/file/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications file get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsFileGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/notifications/file/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications file get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsFileGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/notifications/file/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications file get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsFileGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/notifications/file/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications file get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsImageGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/notifications/image/get" }), zod.z.union([CliErrorSchema, AgentCompletionsMessageImageUrlSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications image get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsImageGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/notifications/image/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications image get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsImageGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/notifications/image/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications image get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsImageGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/notifications/image/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications image get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsImageGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/notifications/image/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications image get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsImageGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/notifications/image/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications image get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsTextGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/notifications/text/get" }), zod.z.union([CliErrorSchema, zod.z.string()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications text get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsTextGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/notifications/text/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications text get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsTextGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/notifications/text/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications text get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsTextGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/notifications/text/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications text get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsTextGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/notifications/text/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications text get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsTextGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/notifications/text/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications text get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsVideoGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/notifications/video/get" }), zod.z.union([CliErrorSchema, AgentCompletionsMessageVideoUrlSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications video get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsVideoGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/notifications/video/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications video get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsVideoGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/notifications/video/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications video get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsVideoGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/notifications/video/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications video get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsVideoGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/notifications/video/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications video get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsVideoGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/notifications/video/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications video get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestSubscribeExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/subscribe" }), zod.z.union([CliErrorSchema, AgentCompletionsRequestAgentCompletionCreateParamsLogSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestSubscribeExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/subscribe" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestSubscribeRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/subscribe/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestSubscribeRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/subscribe/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestSubscribeResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/subscribe/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestSubscribeResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/subscribe/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandLogsAgentsCompletionsResponseClearResponseSchema = zod.z.object({
  count: zod.z.number().int().min(0).max(18446744073709552e3)
}).meta({ title: "cli.command.logs.agents.completions.response.clear.Response" });

// src/viewer/command/logs/agents/completions/response/clear.ts
async function logsAgentsCompletionsResponseClearExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/clear" }), zod.z.union([CliErrorSchema, CliCommandLogsAgentsCompletionsResponseClearResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseClearExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/clear" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseClearRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/clear/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseClearRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/clear/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseClearResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/clear/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseClearResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/clear/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandLogsAgentsCompletionsResponseContinuationsClearResponseSchema = zod.z.object({
  count: zod.z.number().int().min(0).max(18446744073709552e3)
}).meta({ title: "cli.command.logs.agents.completions.response.continuations.clear.Response" });

// src/viewer/command/logs/agents/completions/response/continuations/clear.ts
async function logsAgentsCompletionsResponseContinuationsClearExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/continuations/clear" }), zod.z.union([CliErrorSchema, CliCommandLogsAgentsCompletionsResponseContinuationsClearResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response continuations clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseContinuationsClearExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/continuations/clear" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response continuations clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseContinuationsClearRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/continuations/clear/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response continuations clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseContinuationsClearRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/continuations/clear/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response continuations clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseContinuationsClearResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/continuations/clear/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response continuations clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseContinuationsClearResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/continuations/clear/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response continuations clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseContinuationsGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/continuations/get" }), zod.z.union([CliErrorSchema, zod.z.string()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response continuations get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseContinuationsGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/continuations/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response continuations get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseContinuationsGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/continuations/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response continuations get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseContinuationsGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/continuations/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response continuations get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseContinuationsGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/continuations/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response continuations get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseContinuationsGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/continuations/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response continuations get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseContinuationsSubscribeExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/continuations/subscribe" }), zod.z.union([CliErrorSchema, zod.z.string()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response continuations subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseContinuationsSubscribeExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/continuations/subscribe" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response continuations subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseContinuationsSubscribeRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/continuations/subscribe/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response continuations subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseContinuationsSubscribeRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/continuations/subscribe/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response continuations subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseContinuationsSubscribeResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/continuations/subscribe/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response continuations subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseContinuationsSubscribeResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/continuations/subscribe/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response continuations subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/get" }), zod.z.union([CliErrorSchema, AgentCompletionsResponseStreamingAgentCompletionChunkLogSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandLogsAgentsCompletionsResponseListResponseItemSchema = zod.z.object({
  created: zod.z.number().int().min(0).max(18446744073709552e3),
  id: zod.z.string()
}).meta({ title: "cli.command.logs.agents.completions.response.list.ResponseItem" });

// src/viewer/command/logs/agents/completions/response/list.ts
function logsAgentsCompletionsResponseListExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/list" }), zod.z.union([CliErrorSchema, CliCommandLogsAgentsCompletionsResponseListResponseItemSchema]));
}
function logsAgentsCompletionsResponseListExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/list" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function logsAgentsCompletionsResponseListRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/list/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseListRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/list/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseListResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/list/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response list response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseListResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/list/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response list response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandLogsAgentsCompletionsResponseMessagesAudioClearResponseSchema = zod.z.object({
  count: zod.z.number().int().min(0).max(18446744073709552e3)
}).meta({ title: "cli.command.logs.agents.completions.response.messages.audio.clear.Response" });

// src/viewer/command/logs/agents/completions/response/messages/audio/clear.ts
async function logsAgentsCompletionsResponseMessagesAudioClearExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/audio/clear" }), zod.z.union([CliErrorSchema, CliCommandLogsAgentsCompletionsResponseMessagesAudioClearResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages audio clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAudioClearExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/audio/clear" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages audio clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAudioClearRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/audio/clear/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages audio clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAudioClearRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/audio/clear/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages audio clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAudioClearResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/audio/clear/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages audio clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAudioClearResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/audio/clear/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages audio clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAudioGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/audio/get" }), zod.z.union([CliErrorSchema, AgentCompletionsMessageInputAudioSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages audio get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAudioGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/audio/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages audio get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAudioGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/audio/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages audio get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAudioGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/audio/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages audio get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAudioGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/audio/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages audio get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAudioGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/audio/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages audio get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAudioSubscribeExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/audio/subscribe" }), zod.z.union([CliErrorSchema, AgentCompletionsMessageInputAudioSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages audio subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAudioSubscribeExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/audio/subscribe" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages audio subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAudioSubscribeRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/audio/subscribe/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages audio subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAudioSubscribeRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/audio/subscribe/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages audio subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAudioSubscribeResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/audio/subscribe/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages audio subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAudioSubscribeResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/audio/subscribe/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages audio subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandLogsAgentsCompletionsResponseMessagesClearResponseSchema = zod.z.object({
  count: zod.z.number().int().min(0).max(18446744073709552e3)
}).meta({ title: "cli.command.logs.agents.completions.response.messages.clear.Response" });

// src/viewer/command/logs/agents/completions/response/messages/clear.ts
async function logsAgentsCompletionsResponseMessagesClearExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/clear" }), zod.z.union([CliErrorSchema, CliCommandLogsAgentsCompletionsResponseMessagesClearResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesClearExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/clear" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesClearRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/clear/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesClearRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/clear/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesClearResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/clear/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesClearResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/clear/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandLogsAgentsCompletionsResponseMessagesFileClearResponseSchema = zod.z.object({
  count: zod.z.number().int().min(0).max(18446744073709552e3)
}).meta({ title: "cli.command.logs.agents.completions.response.messages.file.clear.Response" });

// src/viewer/command/logs/agents/completions/response/messages/file/clear.ts
async function logsAgentsCompletionsResponseMessagesFileClearExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/file/clear" }), zod.z.union([CliErrorSchema, CliCommandLogsAgentsCompletionsResponseMessagesFileClearResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages file clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesFileClearExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/file/clear" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages file clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesFileClearRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/file/clear/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages file clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesFileClearRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/file/clear/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages file clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesFileClearResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/file/clear/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages file clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesFileClearResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/file/clear/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages file clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesFileGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/file/get" }), zod.z.union([CliErrorSchema, AgentCompletionsMessageFileSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages file get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesFileGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/file/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages file get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesFileGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/file/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages file get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesFileGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/file/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages file get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesFileGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/file/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages file get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesFileGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/file/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages file get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesFileSubscribeExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/file/subscribe" }), zod.z.union([CliErrorSchema, AgentCompletionsMessageFileSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages file subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesFileSubscribeExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/file/subscribe" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages file subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesFileSubscribeRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/file/subscribe/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages file subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesFileSubscribeRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/file/subscribe/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages file subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesFileSubscribeResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/file/subscribe/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages file subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesFileSubscribeResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/file/subscribe/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages file subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/get" }), zod.z.union([CliErrorSchema, AgentCompletionsMessageMessageLogSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandLogsAgentsCompletionsResponseMessagesImageClearResponseSchema = zod.z.object({
  count: zod.z.number().int().min(0).max(18446744073709552e3)
}).meta({ title: "cli.command.logs.agents.completions.response.messages.image.clear.Response" });

// src/viewer/command/logs/agents/completions/response/messages/image/clear.ts
async function logsAgentsCompletionsResponseMessagesImageClearExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/image/clear" }), zod.z.union([CliErrorSchema, CliCommandLogsAgentsCompletionsResponseMessagesImageClearResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages image clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesImageClearExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/image/clear" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages image clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesImageClearRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/image/clear/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages image clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesImageClearRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/image/clear/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages image clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesImageClearResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/image/clear/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages image clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesImageClearResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/image/clear/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages image clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesImageGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/image/get" }), zod.z.union([CliErrorSchema, AgentCompletionsMessageImageUrlSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages image get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesImageGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/image/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages image get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesImageGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/image/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages image get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesImageGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/image/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages image get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesImageGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/image/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages image get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesImageGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/image/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages image get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesImageSubscribeExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/image/subscribe" }), zod.z.union([CliErrorSchema, AgentCompletionsMessageImageUrlSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages image subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesImageSubscribeExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/image/subscribe" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages image subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesImageSubscribeRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/image/subscribe/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages image subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesImageSubscribeRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/image/subscribe/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages image subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesImageSubscribeResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/image/subscribe/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages image subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesImageSubscribeResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/image/subscribe/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages image subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandLogsAgentsCompletionsResponseMessagesLogprobsClearResponseSchema = zod.z.object({
  count: zod.z.number().int().min(0).max(18446744073709552e3)
}).meta({ title: "cli.command.logs.agents.completions.response.messages.logprobs.clear.Response" });

// src/viewer/command/logs/agents/completions/response/messages/logprobs/clear.ts
async function logsAgentsCompletionsResponseMessagesLogprobsClearExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/logprobs/clear" }), zod.z.union([CliErrorSchema, CliCommandLogsAgentsCompletionsResponseMessagesLogprobsClearResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages logprobs clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesLogprobsClearExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/logprobs/clear" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages logprobs clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesLogprobsClearRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/logprobs/clear/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages logprobs clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesLogprobsClearRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/logprobs/clear/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages logprobs clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesLogprobsClearResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/logprobs/clear/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages logprobs clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesLogprobsClearResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/logprobs/clear/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages logprobs clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesLogprobsGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/logprobs/get" }), zod.z.union([CliErrorSchema, AgentCompletionsResponseLogprobsSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages logprobs get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesLogprobsGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/logprobs/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages logprobs get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesLogprobsGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/logprobs/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages logprobs get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesLogprobsGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/logprobs/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages logprobs get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesLogprobsGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/logprobs/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages logprobs get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesLogprobsGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/logprobs/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages logprobs get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesLogprobsSubscribeExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/logprobs/subscribe" }), zod.z.union([CliErrorSchema, AgentCompletionsResponseLogprobsSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages logprobs subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesLogprobsSubscribeExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/logprobs/subscribe" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages logprobs subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesLogprobsSubscribeRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/logprobs/subscribe/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages logprobs subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesLogprobsSubscribeRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/logprobs/subscribe/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages logprobs subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesLogprobsSubscribeResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/logprobs/subscribe/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages logprobs subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesLogprobsSubscribeResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/logprobs/subscribe/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages logprobs subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandLogsAgentsCompletionsResponseMessagesReasoningClearResponseSchema = zod.z.object({
  count: zod.z.number().int().min(0).max(18446744073709552e3)
}).meta({ title: "cli.command.logs.agents.completions.response.messages.reasoning.clear.Response" });

// src/viewer/command/logs/agents/completions/response/messages/reasoning/clear.ts
async function logsAgentsCompletionsResponseMessagesReasoningClearExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/reasoning/clear" }), zod.z.union([CliErrorSchema, CliCommandLogsAgentsCompletionsResponseMessagesReasoningClearResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages reasoning clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesReasoningClearExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/reasoning/clear" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages reasoning clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesReasoningClearRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/reasoning/clear/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages reasoning clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesReasoningClearRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/reasoning/clear/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages reasoning clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesReasoningClearResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/reasoning/clear/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages reasoning clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesReasoningClearResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/reasoning/clear/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages reasoning clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesReasoningGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/reasoning/get" }), zod.z.union([CliErrorSchema, zod.z.string()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages reasoning get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesReasoningGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/reasoning/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages reasoning get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesReasoningGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/reasoning/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages reasoning get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesReasoningGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/reasoning/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages reasoning get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesReasoningGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/reasoning/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages reasoning get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesReasoningGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/reasoning/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages reasoning get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesReasoningSubscribeExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/reasoning/subscribe" }), zod.z.union([CliErrorSchema, zod.z.string()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages reasoning subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesReasoningSubscribeExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/reasoning/subscribe" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages reasoning subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesReasoningSubscribeRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/reasoning/subscribe/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages reasoning subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesReasoningSubscribeRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/reasoning/subscribe/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages reasoning subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesReasoningSubscribeResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/reasoning/subscribe/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages reasoning subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesReasoningSubscribeResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/reasoning/subscribe/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages reasoning subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandLogsAgentsCompletionsResponseMessagesRefusalClearResponseSchema = zod.z.object({
  count: zod.z.number().int().min(0).max(18446744073709552e3)
}).meta({ title: "cli.command.logs.agents.completions.response.messages.refusal.clear.Response" });

// src/viewer/command/logs/agents/completions/response/messages/refusal/clear.ts
async function logsAgentsCompletionsResponseMessagesRefusalClearExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/refusal/clear" }), zod.z.union([CliErrorSchema, CliCommandLogsAgentsCompletionsResponseMessagesRefusalClearResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages refusal clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesRefusalClearExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/refusal/clear" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages refusal clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesRefusalClearRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/refusal/clear/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages refusal clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesRefusalClearRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/refusal/clear/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages refusal clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesRefusalClearResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/refusal/clear/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages refusal clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesRefusalClearResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/refusal/clear/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages refusal clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesRefusalGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/refusal/get" }), zod.z.union([CliErrorSchema, zod.z.string()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages refusal get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesRefusalGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/refusal/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages refusal get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesRefusalGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/refusal/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages refusal get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesRefusalGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/refusal/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages refusal get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesRefusalGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/refusal/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages refusal get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesRefusalGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/refusal/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages refusal get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesRefusalSubscribeExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/refusal/subscribe" }), zod.z.union([CliErrorSchema, zod.z.string()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages refusal subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesRefusalSubscribeExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/refusal/subscribe" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages refusal subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesRefusalSubscribeRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/refusal/subscribe/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages refusal subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesRefusalSubscribeRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/refusal/subscribe/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages refusal subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesRefusalSubscribeResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/refusal/subscribe/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages refusal subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesRefusalSubscribeResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/refusal/subscribe/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages refusal subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesSubscribeExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/subscribe" }), zod.z.union([CliErrorSchema, AgentCompletionsMessageMessageLogSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesSubscribeExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/subscribe" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesSubscribeRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/subscribe/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesSubscribeRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/subscribe/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesSubscribeResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/subscribe/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesSubscribeResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/subscribe/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesTextGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/text/get" }), zod.z.union([CliErrorSchema, zod.z.string()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages text get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesTextGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/text/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages text get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesTextGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/text/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages text get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesTextGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/text/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages text get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesTextGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/text/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages text get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesTextGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/text/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages text get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolAudioGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/tool/audio/get" }), zod.z.union([CliErrorSchema, AgentCompletionsMessageInputAudioSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool audio get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolAudioGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/tool/audio/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool audio get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolAudioGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/tool/audio/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool audio get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolAudioGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/tool/audio/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool audio get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolAudioGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/tool/audio/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool audio get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolAudioGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/tool/audio/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool audio get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolFileGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/tool/file/get" }), zod.z.union([CliErrorSchema, AgentCompletionsMessageFileSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool file get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolFileGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/tool/file/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool file get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolFileGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/tool/file/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool file get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolFileGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/tool/file/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool file get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolFileGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/tool/file/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool file get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolFileGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/tool/file/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool file get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolImageGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/tool/image/get" }), zod.z.union([CliErrorSchema, AgentCompletionsMessageImageUrlSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool image get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolImageGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/tool/image/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool image get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolImageGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/tool/image/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool image get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolImageGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/tool/image/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool image get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolImageGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/tool/image/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool image get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolImageGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/tool/image/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool image get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolTextGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/tool/text/get" }), zod.z.union([CliErrorSchema, zod.z.string()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool text get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolTextGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/tool/text/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool text get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolTextGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/tool/text/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool text get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolTextGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/tool/text/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool text get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolTextGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/tool/text/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool text get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolTextGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/tool/text/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool text get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolVideoGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/tool/video/get" }), zod.z.union([CliErrorSchema, AgentCompletionsMessageVideoUrlSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool video get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolVideoGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/tool/video/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool video get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolVideoGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/tool/video/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool video get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolVideoGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/tool/video/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool video get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolVideoGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/tool/video/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool video get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolVideoGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/tool/video/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool video get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandLogsAgentsCompletionsResponseMessagesToolCallsClearResponseSchema = zod.z.object({
  count: zod.z.number().int().min(0).max(18446744073709552e3)
}).meta({ title: "cli.command.logs.agents.completions.response.messages.tool_calls.clear.Response" });

// src/viewer/command/logs/agents/completions/response/messages/tool_calls/clear.ts
async function logsAgentsCompletionsResponseMessagesToolCallsClearExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/tool_calls/clear" }), zod.z.union([CliErrorSchema, CliCommandLogsAgentsCompletionsResponseMessagesToolCallsClearResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool_calls clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolCallsClearExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/tool_calls/clear" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool_calls clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolCallsClearRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/tool_calls/clear/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool_calls clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolCallsClearRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/tool_calls/clear/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool_calls clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolCallsClearResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/tool_calls/clear/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool_calls clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolCallsClearResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/tool_calls/clear/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool_calls clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolCallsGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/tool_calls/get" }), zod.z.union([CliErrorSchema, AgentCompletionsMessageAssistantToolCallDeltaSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool_calls get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolCallsGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/tool_calls/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool_calls get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolCallsGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/tool_calls/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool_calls get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolCallsGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/tool_calls/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool_calls get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolCallsGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/tool_calls/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool_calls get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolCallsGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/tool_calls/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool_calls get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolCallsSubscribeExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/tool_calls/subscribe" }), zod.z.union([CliErrorSchema, AgentCompletionsMessageAssistantToolCallDeltaSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool_calls subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolCallsSubscribeExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/tool_calls/subscribe" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool_calls subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolCallsSubscribeRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/tool_calls/subscribe/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool_calls subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolCallsSubscribeRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/tool_calls/subscribe/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool_calls subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolCallsSubscribeResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/tool_calls/subscribe/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool_calls subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolCallsSubscribeResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/tool_calls/subscribe/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool_calls subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandLogsAgentsCompletionsResponseMessagesVideoClearResponseSchema = zod.z.object({
  count: zod.z.number().int().min(0).max(18446744073709552e3)
}).meta({ title: "cli.command.logs.agents.completions.response.messages.video.clear.Response" });

// src/viewer/command/logs/agents/completions/response/messages/video/clear.ts
async function logsAgentsCompletionsResponseMessagesVideoClearExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/video/clear" }), zod.z.union([CliErrorSchema, CliCommandLogsAgentsCompletionsResponseMessagesVideoClearResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages video clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesVideoClearExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/video/clear" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages video clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesVideoClearRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/video/clear/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages video clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesVideoClearRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/video/clear/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages video clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesVideoClearResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/video/clear/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages video clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesVideoClearResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/video/clear/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages video clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesVideoGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/video/get" }), zod.z.union([CliErrorSchema, AgentCompletionsMessageVideoUrlSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages video get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesVideoGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/video/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages video get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesVideoGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/video/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages video get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesVideoGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/video/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages video get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesVideoGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/video/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages video get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesVideoGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/video/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages video get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesVideoSubscribeExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/video/subscribe" }), zod.z.union([CliErrorSchema, AgentCompletionsMessageVideoUrlSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages video subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesVideoSubscribeExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/video/subscribe" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages video subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesVideoSubscribeRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/video/subscribe/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages video subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesVideoSubscribeRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/video/subscribe/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages video subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesVideoSubscribeResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/video/subscribe/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages video subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesVideoSubscribeResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/video/subscribe/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages video subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseSubscribeExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/subscribe" }), zod.z.union([CliErrorSchema, AgentCompletionsResponseStreamingAgentCompletionChunkLogSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseSubscribeExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/subscribe" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseSubscribeRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/subscribe/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseSubscribeRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/subscribe/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseSubscribeResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/subscribe/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseSubscribeResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/subscribe/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandLogsClearResponseSchema = zod.z.object({
  count: zod.z.number().int().min(0).max(18446744073709552e3)
}).meta({ title: "cli.command.logs.clear.Response" });

// src/viewer/command/logs/clear.ts
async function logsClearExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/clear" }), zod.z.union([CliErrorSchema, CliCommandLogsClearResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsClearExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/clear" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsClearRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/clear/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsClearRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/clear/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsClearResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/clear/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsClearResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/clear/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsRequestGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/request/get" }), zod.z.union([CliErrorSchema, FunctionsExecutionsRequestFunctionExecutionCreateParamsLogSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions request get: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsRequestGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/request/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions request get: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsRequestGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/request/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions request get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsRequestGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/request/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions request get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsRequestGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/request/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions request get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsRequestGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/request/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions request get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsRequestSubscribeExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/request/subscribe" }), zod.z.union([CliErrorSchema, FunctionsExecutionsRequestFunctionExecutionCreateParamsLogSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions request subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsRequestSubscribeExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/request/subscribe" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions request subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsRequestSubscribeRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/request/subscribe/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions request subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsRequestSubscribeRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/request/subscribe/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions request subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsRequestSubscribeResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/request/subscribe/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions request subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsRequestSubscribeResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/request/subscribe/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions request subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandLogsFunctionsExecutionsResponseClearResponseSchema = zod.z.object({
  count: zod.z.number().int().min(0).max(18446744073709552e3)
}).meta({ title: "cli.command.logs.functions.executions.response.clear.Response" });

// src/viewer/command/logs/functions/executions/response/clear.ts
async function logsFunctionsExecutionsResponseClearExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/response/clear" }), zod.z.union([CliErrorSchema, CliCommandLogsFunctionsExecutionsResponseClearResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseClearExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/response/clear" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseClearRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/response/clear/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseClearRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/response/clear/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseClearResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/response/clear/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseClearResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/response/clear/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/response/get" }), zod.z.union([CliErrorSchema, FunctionsExecutionsResponseStreamingFunctionExecutionChunkLogSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response get: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/response/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response get: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/response/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/response/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/response/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/response/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandLogsFunctionsExecutionsResponseListResponseItemSchema = zod.z.object({
  created: zod.z.number().int().min(0).max(18446744073709552e3),
  id: zod.z.string()
}).meta({ title: "cli.command.logs.functions.executions.response.list.ResponseItem" });

// src/viewer/command/logs/functions/executions/response/list.ts
function logsFunctionsExecutionsResponseListExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/response/list" }), zod.z.union([CliErrorSchema, CliCommandLogsFunctionsExecutionsResponseListResponseItemSchema]));
}
function logsFunctionsExecutionsResponseListExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/response/list" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function logsFunctionsExecutionsResponseListRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/response/list/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseListRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/response/list/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseListResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/response/list/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response list response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseListResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/response/list/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response list response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandLogsFunctionsExecutionsResponseRetryTokensClearResponseSchema = zod.z.object({
  count: zod.z.number().int().min(0).max(18446744073709552e3)
}).meta({ title: "cli.command.logs.functions.executions.response.retry_tokens.clear.Response" });

// src/viewer/command/logs/functions/executions/response/retry_tokens/clear.ts
async function logsFunctionsExecutionsResponseRetryTokensClearExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/response/retry_tokens/clear" }), zod.z.union([CliErrorSchema, CliCommandLogsFunctionsExecutionsResponseRetryTokensClearResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response retry_tokens clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseRetryTokensClearExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/response/retry_tokens/clear" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response retry_tokens clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseRetryTokensClearRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/response/retry_tokens/clear/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response retry_tokens clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseRetryTokensClearRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/response/retry_tokens/clear/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response retry_tokens clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseRetryTokensClearResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/response/retry_tokens/clear/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response retry_tokens clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseRetryTokensClearResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/response/retry_tokens/clear/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response retry_tokens clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseRetryTokensGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/response/retry_tokens/get" }), zod.z.union([CliErrorSchema, zod.z.string()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response retry_tokens get: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseRetryTokensGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/response/retry_tokens/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response retry_tokens get: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseRetryTokensGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/response/retry_tokens/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response retry_tokens get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseRetryTokensGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/response/retry_tokens/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response retry_tokens get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseRetryTokensGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/response/retry_tokens/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response retry_tokens get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseRetryTokensGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/response/retry_tokens/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response retry_tokens get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseRetryTokensSubscribeExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/response/retry_tokens/subscribe" }), zod.z.union([CliErrorSchema, zod.z.string()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response retry_tokens subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseRetryTokensSubscribeExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/response/retry_tokens/subscribe" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response retry_tokens subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseRetryTokensSubscribeRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/response/retry_tokens/subscribe/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response retry_tokens subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseRetryTokensSubscribeRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/response/retry_tokens/subscribe/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response retry_tokens subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseRetryTokensSubscribeResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/response/retry_tokens/subscribe/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response retry_tokens subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseRetryTokensSubscribeResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/response/retry_tokens/subscribe/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response retry_tokens subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseSubscribeExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/response/subscribe" }), zod.z.union([CliErrorSchema, FunctionsExecutionsResponseStreamingFunctionExecutionChunkLogSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseSubscribeExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/response/subscribe" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseSubscribeRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/response/subscribe/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseSubscribeRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/response/subscribe/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseSubscribeResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/response/subscribe/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseSubscribeResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/response/subscribe/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveRequestGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/recursive/request/get" }), zod.z.union([CliErrorSchema, FunctionsInventionsRecursiveRequestFunctionInventionRecursiveCreateParamsLogSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive request get: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveRequestGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/recursive/request/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive request get: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveRequestGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/recursive/request/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive request get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveRequestGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/recursive/request/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive request get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveRequestGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/recursive/request/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive request get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveRequestGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/recursive/request/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive request get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveRequestSubscribeExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/recursive/request/subscribe" }), zod.z.union([CliErrorSchema, FunctionsInventionsRecursiveRequestFunctionInventionRecursiveCreateParamsLogSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive request subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveRequestSubscribeExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/recursive/request/subscribe" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive request subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveRequestSubscribeRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/recursive/request/subscribe/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive request subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveRequestSubscribeRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/recursive/request/subscribe/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive request subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveRequestSubscribeResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/recursive/request/subscribe/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive request subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveRequestSubscribeResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/recursive/request/subscribe/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive request subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandLogsFunctionsInventionsRecursiveResponseClearResponseSchema = zod.z.object({
  count: zod.z.number().int().min(0).max(18446744073709552e3)
}).meta({ title: "cli.command.logs.functions.inventions.recursive.response.clear.Response" });

// src/viewer/command/logs/functions/inventions/recursive/response/clear.ts
async function logsFunctionsInventionsRecursiveResponseClearExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/recursive/response/clear" }), zod.z.union([CliErrorSchema, CliCommandLogsFunctionsInventionsRecursiveResponseClearResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive response clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveResponseClearExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/recursive/response/clear" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive response clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveResponseClearRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/recursive/response/clear/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive response clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveResponseClearRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/recursive/response/clear/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive response clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveResponseClearResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/recursive/response/clear/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive response clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveResponseClearResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/recursive/response/clear/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive response clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveResponseGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/recursive/response/get" }), zod.z.union([CliErrorSchema, FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunkLogSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive response get: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveResponseGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/recursive/response/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive response get: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveResponseGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/recursive/response/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive response get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveResponseGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/recursive/response/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive response get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveResponseGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/recursive/response/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive response get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveResponseGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/recursive/response/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive response get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandLogsFunctionsInventionsRecursiveResponseListResponseItemSchema = zod.z.object({
  created: zod.z.number().int().min(0).max(18446744073709552e3),
  id: zod.z.string()
}).meta({ title: "cli.command.logs.functions.inventions.recursive.response.list.ResponseItem" });

// src/viewer/command/logs/functions/inventions/recursive/response/list.ts
function logsFunctionsInventionsRecursiveResponseListExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/recursive/response/list" }), zod.z.union([CliErrorSchema, CliCommandLogsFunctionsInventionsRecursiveResponseListResponseItemSchema]));
}
function logsFunctionsInventionsRecursiveResponseListExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/recursive/response/list" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function logsFunctionsInventionsRecursiveResponseListRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/recursive/response/list/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive response list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveResponseListRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/recursive/response/list/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive response list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveResponseListResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/recursive/response/list/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive response list response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveResponseListResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/recursive/response/list/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive response list response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveResponseSubscribeExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/recursive/response/subscribe" }), zod.z.union([CliErrorSchema, FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunkLogSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive response subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveResponseSubscribeExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/recursive/response/subscribe" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive response subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveResponseSubscribeRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/recursive/response/subscribe/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive response subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveResponseSubscribeRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/recursive/response/subscribe/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive response subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveResponseSubscribeResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/recursive/response/subscribe/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive response subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveResponseSubscribeResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/recursive/response/subscribe/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive response subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRequestGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/request/get" }), zod.z.union([CliErrorSchema, FunctionsInventionsRequestFunctionInventionCreateParamsSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions request get: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRequestGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/request/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions request get: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRequestGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/request/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions request get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRequestGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/request/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions request get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRequestGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/request/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions request get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRequestGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/request/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions request get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRequestSubscribeExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/request/subscribe" }), zod.z.union([CliErrorSchema, FunctionsInventionsRequestFunctionInventionCreateParamsSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions request subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRequestSubscribeExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/request/subscribe" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions request subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRequestSubscribeRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/request/subscribe/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions request subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRequestSubscribeRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/request/subscribe/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions request subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRequestSubscribeResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/request/subscribe/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions request subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRequestSubscribeResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/request/subscribe/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions request subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandLogsFunctionsInventionsResponseClearResponseSchema = zod.z.object({
  count: zod.z.number().int().min(0).max(18446744073709552e3)
}).meta({ title: "cli.command.logs.functions.inventions.response.clear.Response" });

// src/viewer/command/logs/functions/inventions/response/clear.ts
async function logsFunctionsInventionsResponseClearExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/response/clear" }), zod.z.union([CliErrorSchema, CliCommandLogsFunctionsInventionsResponseClearResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions response clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsResponseClearExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/response/clear" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions response clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsResponseClearRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/response/clear/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions response clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsResponseClearRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/response/clear/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions response clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsResponseClearResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/response/clear/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions response clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsResponseClearResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/response/clear/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions response clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsResponseGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/response/get" }), zod.z.union([CliErrorSchema, FunctionsInventionsResponseStreamingFunctionInventionChunkLogSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions response get: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsResponseGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/response/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions response get: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsResponseGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/response/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions response get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsResponseGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/response/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions response get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsResponseGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/response/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions response get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsResponseGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/response/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions response get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandLogsFunctionsInventionsResponseListResponseItemSchema = zod.z.object({
  created: zod.z.number().int().min(0).max(18446744073709552e3),
  id: zod.z.string()
}).meta({ title: "cli.command.logs.functions.inventions.response.list.ResponseItem" });

// src/viewer/command/logs/functions/inventions/response/list.ts
function logsFunctionsInventionsResponseListExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/response/list" }), zod.z.union([CliErrorSchema, CliCommandLogsFunctionsInventionsResponseListResponseItemSchema]));
}
function logsFunctionsInventionsResponseListExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/response/list" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function logsFunctionsInventionsResponseListRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/response/list/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions response list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsResponseListRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/response/list/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions response list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsResponseListResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/response/list/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions response list response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsResponseListResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/response/list/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions response list response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsResponseSubscribeExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/response/subscribe" }), zod.z.union([CliErrorSchema, FunctionsInventionsResponseStreamingFunctionInventionChunkLogSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions response subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsResponseSubscribeExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/response/subscribe" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions response subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsResponseSubscribeRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/response/subscribe/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions response subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsResponseSubscribeRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/response/subscribe/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions response subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsResponseSubscribeResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/response/subscribe/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions response subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsResponseSubscribeResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/response/subscribe/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions response subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsRequestGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/vector/completions/request/get" }), zod.z.union([CliErrorSchema, VectorCompletionsRequestVectorCompletionCreateParamsSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions request get: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsRequestGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/vector/completions/request/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions request get: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsRequestGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/vector/completions/request/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions request get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsRequestGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/vector/completions/request/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions request get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsRequestGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/vector/completions/request/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions request get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsRequestGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/vector/completions/request/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions request get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsRequestSubscribeExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/vector/completions/request/subscribe" }), zod.z.union([CliErrorSchema, VectorCompletionsRequestVectorCompletionCreateParamsSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions request subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsRequestSubscribeExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/vector/completions/request/subscribe" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions request subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsRequestSubscribeRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/vector/completions/request/subscribe/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions request subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsRequestSubscribeRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/vector/completions/request/subscribe/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions request subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsRequestSubscribeResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/vector/completions/request/subscribe/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions request subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsRequestSubscribeResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/vector/completions/request/subscribe/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions request subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandLogsVectorCompletionsResponseClearResponseSchema = zod.z.object({
  count: zod.z.number().int().min(0).max(18446744073709552e3)
}).meta({ title: "cli.command.logs.vector.completions.response.clear.Response" });

// src/viewer/command/logs/vector/completions/response/clear.ts
async function logsVectorCompletionsResponseClearExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/vector/completions/response/clear" }), zod.z.union([CliErrorSchema, CliCommandLogsVectorCompletionsResponseClearResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions response clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsResponseClearExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/vector/completions/response/clear" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions response clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsResponseClearRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/vector/completions/response/clear/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions response clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsResponseClearRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/vector/completions/response/clear/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions response clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsResponseClearResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/vector/completions/response/clear/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions response clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsResponseClearResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/vector/completions/response/clear/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions response clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsResponseGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/vector/completions/response/get" }), zod.z.union([CliErrorSchema, VectorCompletionsResponseStreamingVectorCompletionChunkLogSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions response get: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsResponseGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/vector/completions/response/get" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions response get: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsResponseGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/vector/completions/response/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions response get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsResponseGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/vector/completions/response/get/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions response get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsResponseGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/vector/completions/response/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions response get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsResponseGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/vector/completions/response/get/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions response get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandLogsVectorCompletionsResponseListResponseItemSchema = zod.z.object({
  created: zod.z.number().int().min(0).max(18446744073709552e3),
  id: zod.z.string()
}).meta({ title: "cli.command.logs.vector.completions.response.list.ResponseItem" });

// src/viewer/command/logs/vector/completions/response/list.ts
function logsVectorCompletionsResponseListExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/vector/completions/response/list" }), zod.z.union([CliErrorSchema, CliCommandLogsVectorCompletionsResponseListResponseItemSchema]));
}
function logsVectorCompletionsResponseListExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/vector/completions/response/list" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
}
async function logsVectorCompletionsResponseListRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/vector/completions/response/list/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions response list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsResponseListRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/vector/completions/response/list/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions response list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsResponseListResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/vector/completions/response/list/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions response list response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsResponseListResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/vector/completions/response/list/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions response list response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsResponseSubscribeExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/vector/completions/response/subscribe" }), zod.z.union([CliErrorSchema, VectorCompletionsResponseStreamingVectorCompletionChunkLogSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions response subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsResponseSubscribeExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/vector/completions/response/subscribe" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions response subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsResponseSubscribeRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/vector/completions/response/subscribe/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions response subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsResponseSubscribeRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/vector/completions/response/subscribe/request_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions response subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsResponseSubscribeResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/vector/completions/response/subscribe/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions response subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsResponseSubscribeResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/vector/completions/response/subscribe/response_schema" }), zod.z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions response subscribe response_schema: cli produced no output before the end marker");
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
var CliCommandToolsGetResponseManifestSchema = zod.z.object({
  description: zod.z.string(),
  exec: zod.z.string(),
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
exports.agentsListActiveExecute = agentsListActiveExecute;
exports.agentsListActiveExecuteJq = agentsListActiveExecuteJq;
exports.agentsListActiveRequestSchemaExecute = agentsListActiveRequestSchemaExecute;
exports.agentsListActiveRequestSchemaExecuteJq = agentsListActiveRequestSchemaExecuteJq;
exports.agentsListActiveResponseSchemaExecute = agentsListActiveResponseSchemaExecute;
exports.agentsListActiveResponseSchemaExecuteJq = agentsListActiveResponseSchemaExecuteJq;
exports.agentsListAvailableExecute = agentsListAvailableExecute;
exports.agentsListAvailableExecuteJq = agentsListAvailableExecuteJq;
exports.agentsListAvailableRequestSchemaExecute = agentsListAvailableRequestSchemaExecute;
exports.agentsListAvailableRequestSchemaExecuteJq = agentsListAvailableRequestSchemaExecuteJq;
exports.agentsListAvailableResponseSchemaExecute = agentsListAvailableResponseSchemaExecute;
exports.agentsListAvailableResponseSchemaExecuteJq = agentsListAvailableResponseSchemaExecuteJq;
exports.agentsMeExecute = agentsMeExecute;
exports.agentsMeExecuteJq = agentsMeExecuteJq;
exports.agentsMeRequestSchemaExecute = agentsMeRequestSchemaExecute;
exports.agentsMeRequestSchemaExecuteJq = agentsMeRequestSchemaExecuteJq;
exports.agentsMeResponseSchemaExecute = agentsMeResponseSchemaExecute;
exports.agentsMeResponseSchemaExecuteJq = agentsMeResponseSchemaExecuteJq;
exports.agentsMessageExecute = agentsMessageExecute;
exports.agentsMessageExecuteJq = agentsMessageExecuteJq;
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
exports.agentsReadAllExecute = agentsReadAllExecute;
exports.agentsReadAllExecuteJq = agentsReadAllExecuteJq;
exports.agentsReadAllRequestSchemaExecute = agentsReadAllRequestSchemaExecute;
exports.agentsReadAllRequestSchemaExecuteJq = agentsReadAllRequestSchemaExecuteJq;
exports.agentsReadAllResponseSchemaExecute = agentsReadAllResponseSchemaExecute;
exports.agentsReadAllResponseSchemaExecuteJq = agentsReadAllResponseSchemaExecuteJq;
exports.agentsReadIdExecute = agentsReadIdExecute;
exports.agentsReadIdExecuteJq = agentsReadIdExecuteJq;
exports.agentsReadIdRequestSchemaExecute = agentsReadIdRequestSchemaExecute;
exports.agentsReadIdRequestSchemaExecuteJq = agentsReadIdRequestSchemaExecuteJq;
exports.agentsReadIdResponseSchemaExecute = agentsReadIdResponseSchemaExecute;
exports.agentsReadIdResponseSchemaExecuteJq = agentsReadIdResponseSchemaExecuteJq;
exports.agentsReadPendingExecute = agentsReadPendingExecute;
exports.agentsReadPendingExecuteJq = agentsReadPendingExecuteJq;
exports.agentsReadPendingRequestSchemaExecute = agentsReadPendingRequestSchemaExecute;
exports.agentsReadPendingRequestSchemaExecuteJq = agentsReadPendingRequestSchemaExecuteJq;
exports.agentsReadPendingResponseSchemaExecute = agentsReadPendingResponseSchemaExecute;
exports.agentsReadPendingResponseSchemaExecuteJq = agentsReadPendingResponseSchemaExecuteJq;
exports.agentsReadSubscribeExecute = agentsReadSubscribeExecute;
exports.agentsReadSubscribeExecuteJq = agentsReadSubscribeExecuteJq;
exports.agentsReadSubscribeRequestSchemaExecute = agentsReadSubscribeRequestSchemaExecute;
exports.agentsReadSubscribeRequestSchemaExecuteJq = agentsReadSubscribeRequestSchemaExecuteJq;
exports.agentsReadSubscribeResponseSchemaExecute = agentsReadSubscribeResponseSchemaExecute;
exports.agentsReadSubscribeResponseSchemaExecuteJq = agentsReadSubscribeResponseSchemaExecuteJq;
exports.agentsSpawnExecute = agentsSpawnExecute;
exports.agentsSpawnExecuteJq = agentsSpawnExecuteJq;
exports.agentsSpawnExecuteStreaming = agentsSpawnExecuteStreaming;
exports.agentsSpawnExecuteStreamingJq = agentsSpawnExecuteStreamingJq;
exports.agentsSpawnRequestSchemaExecute = agentsSpawnRequestSchemaExecute;
exports.agentsSpawnRequestSchemaExecuteJq = agentsSpawnRequestSchemaExecuteJq;
exports.agentsSpawnResponseSchemaExecute = agentsSpawnResponseSchemaExecute;
exports.agentsSpawnResponseSchemaExecuteJq = agentsSpawnResponseSchemaExecuteJq;
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
exports.configFunctionsInventionsGetExecute = configFunctionsInventionsGetExecute;
exports.configFunctionsInventionsGetExecuteJq = configFunctionsInventionsGetExecuteJq;
exports.configFunctionsInventionsGetRequestSchemaExecute = configFunctionsInventionsGetRequestSchemaExecute;
exports.configFunctionsInventionsGetRequestSchemaExecuteJq = configFunctionsInventionsGetRequestSchemaExecuteJq;
exports.configFunctionsInventionsGetResponseSchemaExecute = configFunctionsInventionsGetResponseSchemaExecute;
exports.configFunctionsInventionsGetResponseSchemaExecuteJq = configFunctionsInventionsGetResponseSchemaExecuteJq;
exports.configFunctionsInventionsRemoteGetExecute = configFunctionsInventionsRemoteGetExecute;
exports.configFunctionsInventionsRemoteGetExecuteJq = configFunctionsInventionsRemoteGetExecuteJq;
exports.configFunctionsInventionsRemoteGetRequestSchemaExecute = configFunctionsInventionsRemoteGetRequestSchemaExecute;
exports.configFunctionsInventionsRemoteGetRequestSchemaExecuteJq = configFunctionsInventionsRemoteGetRequestSchemaExecuteJq;
exports.configFunctionsInventionsRemoteGetResponseSchemaExecute = configFunctionsInventionsRemoteGetResponseSchemaExecute;
exports.configFunctionsInventionsRemoteGetResponseSchemaExecuteJq = configFunctionsInventionsRemoteGetResponseSchemaExecuteJq;
exports.configFunctionsInventionsRemoteSetExecute = configFunctionsInventionsRemoteSetExecute;
exports.configFunctionsInventionsRemoteSetExecuteJq = configFunctionsInventionsRemoteSetExecuteJq;
exports.configFunctionsInventionsRemoteSetRequestSchemaExecute = configFunctionsInventionsRemoteSetRequestSchemaExecute;
exports.configFunctionsInventionsRemoteSetRequestSchemaExecuteJq = configFunctionsInventionsRemoteSetRequestSchemaExecuteJq;
exports.configFunctionsInventionsRemoteSetResponseSchemaExecute = configFunctionsInventionsRemoteSetResponseSchemaExecute;
exports.configFunctionsInventionsRemoteSetResponseSchemaExecuteJq = configFunctionsInventionsRemoteSetResponseSchemaExecuteJq;
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
exports.functionsExecutionsCreateStandardExecute = functionsExecutionsCreateStandardExecute;
exports.functionsExecutionsCreateStandardExecuteJq = functionsExecutionsCreateStandardExecuteJq;
exports.functionsExecutionsCreateStandardExecuteStreaming = functionsExecutionsCreateStandardExecuteStreaming;
exports.functionsExecutionsCreateStandardExecuteStreamingJq = functionsExecutionsCreateStandardExecuteStreamingJq;
exports.functionsExecutionsCreateStandardRequestSchemaExecute = functionsExecutionsCreateStandardRequestSchemaExecute;
exports.functionsExecutionsCreateStandardRequestSchemaExecuteJq = functionsExecutionsCreateStandardRequestSchemaExecuteJq;
exports.functionsExecutionsCreateStandardResponseSchemaExecute = functionsExecutionsCreateStandardResponseSchemaExecute;
exports.functionsExecutionsCreateStandardResponseSchemaExecuteJq = functionsExecutionsCreateStandardResponseSchemaExecuteJq;
exports.functionsExecutionsCreateSwissSystemExecute = functionsExecutionsCreateSwissSystemExecute;
exports.functionsExecutionsCreateSwissSystemExecuteJq = functionsExecutionsCreateSwissSystemExecuteJq;
exports.functionsExecutionsCreateSwissSystemExecuteStreaming = functionsExecutionsCreateSwissSystemExecuteStreaming;
exports.functionsExecutionsCreateSwissSystemExecuteStreamingJq = functionsExecutionsCreateSwissSystemExecuteStreamingJq;
exports.functionsExecutionsCreateSwissSystemRequestSchemaExecute = functionsExecutionsCreateSwissSystemRequestSchemaExecute;
exports.functionsExecutionsCreateSwissSystemRequestSchemaExecuteJq = functionsExecutionsCreateSwissSystemRequestSchemaExecuteJq;
exports.functionsExecutionsCreateSwissSystemResponseSchemaExecute = functionsExecutionsCreateSwissSystemResponseSchemaExecute;
exports.functionsExecutionsCreateSwissSystemResponseSchemaExecuteJq = functionsExecutionsCreateSwissSystemResponseSchemaExecuteJq;
exports.functionsGetExecute = functionsGetExecute;
exports.functionsGetExecuteJq = functionsGetExecuteJq;
exports.functionsGetRequestSchemaExecute = functionsGetRequestSchemaExecute;
exports.functionsGetRequestSchemaExecuteJq = functionsGetRequestSchemaExecuteJq;
exports.functionsGetResponseSchemaExecute = functionsGetResponseSchemaExecute;
exports.functionsGetResponseSchemaExecuteJq = functionsGetResponseSchemaExecuteJq;
exports.functionsInventionsRecursiveCreateAlphaScalarExecute = functionsInventionsRecursiveCreateAlphaScalarExecute;
exports.functionsInventionsRecursiveCreateAlphaScalarExecuteJq = functionsInventionsRecursiveCreateAlphaScalarExecuteJq;
exports.functionsInventionsRecursiveCreateAlphaScalarExecuteStreaming = functionsInventionsRecursiveCreateAlphaScalarExecuteStreaming;
exports.functionsInventionsRecursiveCreateAlphaScalarExecuteStreamingJq = functionsInventionsRecursiveCreateAlphaScalarExecuteStreamingJq;
exports.functionsInventionsRecursiveCreateAlphaScalarRequestSchemaExecute = functionsInventionsRecursiveCreateAlphaScalarRequestSchemaExecute;
exports.functionsInventionsRecursiveCreateAlphaScalarRequestSchemaExecuteJq = functionsInventionsRecursiveCreateAlphaScalarRequestSchemaExecuteJq;
exports.functionsInventionsRecursiveCreateAlphaScalarResponseSchemaExecute = functionsInventionsRecursiveCreateAlphaScalarResponseSchemaExecute;
exports.functionsInventionsRecursiveCreateAlphaScalarResponseSchemaExecuteJq = functionsInventionsRecursiveCreateAlphaScalarResponseSchemaExecuteJq;
exports.functionsInventionsRecursiveCreateAlphaVectorExecute = functionsInventionsRecursiveCreateAlphaVectorExecute;
exports.functionsInventionsRecursiveCreateAlphaVectorExecuteJq = functionsInventionsRecursiveCreateAlphaVectorExecuteJq;
exports.functionsInventionsRecursiveCreateAlphaVectorExecuteStreaming = functionsInventionsRecursiveCreateAlphaVectorExecuteStreaming;
exports.functionsInventionsRecursiveCreateAlphaVectorExecuteStreamingJq = functionsInventionsRecursiveCreateAlphaVectorExecuteStreamingJq;
exports.functionsInventionsRecursiveCreateAlphaVectorRequestSchemaExecute = functionsInventionsRecursiveCreateAlphaVectorRequestSchemaExecute;
exports.functionsInventionsRecursiveCreateAlphaVectorRequestSchemaExecuteJq = functionsInventionsRecursiveCreateAlphaVectorRequestSchemaExecuteJq;
exports.functionsInventionsRecursiveCreateAlphaVectorResponseSchemaExecute = functionsInventionsRecursiveCreateAlphaVectorResponseSchemaExecute;
exports.functionsInventionsRecursiveCreateAlphaVectorResponseSchemaExecuteJq = functionsInventionsRecursiveCreateAlphaVectorResponseSchemaExecuteJq;
exports.functionsInventionsRecursiveCreateRemoteExecute = functionsInventionsRecursiveCreateRemoteExecute;
exports.functionsInventionsRecursiveCreateRemoteExecuteJq = functionsInventionsRecursiveCreateRemoteExecuteJq;
exports.functionsInventionsRecursiveCreateRemoteExecuteStreaming = functionsInventionsRecursiveCreateRemoteExecuteStreaming;
exports.functionsInventionsRecursiveCreateRemoteExecuteStreamingJq = functionsInventionsRecursiveCreateRemoteExecuteStreamingJq;
exports.functionsInventionsRecursiveCreateRemoteRequestSchemaExecute = functionsInventionsRecursiveCreateRemoteRequestSchemaExecute;
exports.functionsInventionsRecursiveCreateRemoteRequestSchemaExecuteJq = functionsInventionsRecursiveCreateRemoteRequestSchemaExecuteJq;
exports.functionsInventionsRecursiveCreateRemoteResponseSchemaExecute = functionsInventionsRecursiveCreateRemoteResponseSchemaExecute;
exports.functionsInventionsRecursiveCreateRemoteResponseSchemaExecuteJq = functionsInventionsRecursiveCreateRemoteResponseSchemaExecuteJq;
exports.functionsInventionsStateGetExecute = functionsInventionsStateGetExecute;
exports.functionsInventionsStateGetExecuteJq = functionsInventionsStateGetExecuteJq;
exports.functionsInventionsStateGetRequestSchemaExecute = functionsInventionsStateGetRequestSchemaExecute;
exports.functionsInventionsStateGetRequestSchemaExecuteJq = functionsInventionsStateGetRequestSchemaExecuteJq;
exports.functionsInventionsStateGetResponseSchemaExecute = functionsInventionsStateGetResponseSchemaExecute;
exports.functionsInventionsStateGetResponseSchemaExecuteJq = functionsInventionsStateGetResponseSchemaExecuteJq;
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
exports.logsAgentsCompletionsRequestGetExecute = logsAgentsCompletionsRequestGetExecute;
exports.logsAgentsCompletionsRequestGetExecuteJq = logsAgentsCompletionsRequestGetExecuteJq;
exports.logsAgentsCompletionsRequestGetRequestSchemaExecute = logsAgentsCompletionsRequestGetRequestSchemaExecute;
exports.logsAgentsCompletionsRequestGetRequestSchemaExecuteJq = logsAgentsCompletionsRequestGetRequestSchemaExecuteJq;
exports.logsAgentsCompletionsRequestGetResponseSchemaExecute = logsAgentsCompletionsRequestGetResponseSchemaExecute;
exports.logsAgentsCompletionsRequestGetResponseSchemaExecuteJq = logsAgentsCompletionsRequestGetResponseSchemaExecuteJq;
exports.logsAgentsCompletionsRequestMessagesAudioGetExecute = logsAgentsCompletionsRequestMessagesAudioGetExecute;
exports.logsAgentsCompletionsRequestMessagesAudioGetExecuteJq = logsAgentsCompletionsRequestMessagesAudioGetExecuteJq;
exports.logsAgentsCompletionsRequestMessagesAudioGetRequestSchemaExecute = logsAgentsCompletionsRequestMessagesAudioGetRequestSchemaExecute;
exports.logsAgentsCompletionsRequestMessagesAudioGetRequestSchemaExecuteJq = logsAgentsCompletionsRequestMessagesAudioGetRequestSchemaExecuteJq;
exports.logsAgentsCompletionsRequestMessagesAudioGetResponseSchemaExecute = logsAgentsCompletionsRequestMessagesAudioGetResponseSchemaExecute;
exports.logsAgentsCompletionsRequestMessagesAudioGetResponseSchemaExecuteJq = logsAgentsCompletionsRequestMessagesAudioGetResponseSchemaExecuteJq;
exports.logsAgentsCompletionsRequestMessagesFileGetExecute = logsAgentsCompletionsRequestMessagesFileGetExecute;
exports.logsAgentsCompletionsRequestMessagesFileGetExecuteJq = logsAgentsCompletionsRequestMessagesFileGetExecuteJq;
exports.logsAgentsCompletionsRequestMessagesFileGetRequestSchemaExecute = logsAgentsCompletionsRequestMessagesFileGetRequestSchemaExecute;
exports.logsAgentsCompletionsRequestMessagesFileGetRequestSchemaExecuteJq = logsAgentsCompletionsRequestMessagesFileGetRequestSchemaExecuteJq;
exports.logsAgentsCompletionsRequestMessagesFileGetResponseSchemaExecute = logsAgentsCompletionsRequestMessagesFileGetResponseSchemaExecute;
exports.logsAgentsCompletionsRequestMessagesFileGetResponseSchemaExecuteJq = logsAgentsCompletionsRequestMessagesFileGetResponseSchemaExecuteJq;
exports.logsAgentsCompletionsRequestMessagesImageGetExecute = logsAgentsCompletionsRequestMessagesImageGetExecute;
exports.logsAgentsCompletionsRequestMessagesImageGetExecuteJq = logsAgentsCompletionsRequestMessagesImageGetExecuteJq;
exports.logsAgentsCompletionsRequestMessagesImageGetRequestSchemaExecute = logsAgentsCompletionsRequestMessagesImageGetRequestSchemaExecute;
exports.logsAgentsCompletionsRequestMessagesImageGetRequestSchemaExecuteJq = logsAgentsCompletionsRequestMessagesImageGetRequestSchemaExecuteJq;
exports.logsAgentsCompletionsRequestMessagesImageGetResponseSchemaExecute = logsAgentsCompletionsRequestMessagesImageGetResponseSchemaExecute;
exports.logsAgentsCompletionsRequestMessagesImageGetResponseSchemaExecuteJq = logsAgentsCompletionsRequestMessagesImageGetResponseSchemaExecuteJq;
exports.logsAgentsCompletionsRequestMessagesTextGetExecute = logsAgentsCompletionsRequestMessagesTextGetExecute;
exports.logsAgentsCompletionsRequestMessagesTextGetExecuteJq = logsAgentsCompletionsRequestMessagesTextGetExecuteJq;
exports.logsAgentsCompletionsRequestMessagesTextGetRequestSchemaExecute = logsAgentsCompletionsRequestMessagesTextGetRequestSchemaExecute;
exports.logsAgentsCompletionsRequestMessagesTextGetRequestSchemaExecuteJq = logsAgentsCompletionsRequestMessagesTextGetRequestSchemaExecuteJq;
exports.logsAgentsCompletionsRequestMessagesTextGetResponseSchemaExecute = logsAgentsCompletionsRequestMessagesTextGetResponseSchemaExecute;
exports.logsAgentsCompletionsRequestMessagesTextGetResponseSchemaExecuteJq = logsAgentsCompletionsRequestMessagesTextGetResponseSchemaExecuteJq;
exports.logsAgentsCompletionsRequestMessagesVideoGetExecute = logsAgentsCompletionsRequestMessagesVideoGetExecute;
exports.logsAgentsCompletionsRequestMessagesVideoGetExecuteJq = logsAgentsCompletionsRequestMessagesVideoGetExecuteJq;
exports.logsAgentsCompletionsRequestMessagesVideoGetRequestSchemaExecute = logsAgentsCompletionsRequestMessagesVideoGetRequestSchemaExecute;
exports.logsAgentsCompletionsRequestMessagesVideoGetRequestSchemaExecuteJq = logsAgentsCompletionsRequestMessagesVideoGetRequestSchemaExecuteJq;
exports.logsAgentsCompletionsRequestMessagesVideoGetResponseSchemaExecute = logsAgentsCompletionsRequestMessagesVideoGetResponseSchemaExecute;
exports.logsAgentsCompletionsRequestMessagesVideoGetResponseSchemaExecuteJq = logsAgentsCompletionsRequestMessagesVideoGetResponseSchemaExecuteJq;
exports.logsAgentsCompletionsRequestNotificationsAudioGetExecute = logsAgentsCompletionsRequestNotificationsAudioGetExecute;
exports.logsAgentsCompletionsRequestNotificationsAudioGetExecuteJq = logsAgentsCompletionsRequestNotificationsAudioGetExecuteJq;
exports.logsAgentsCompletionsRequestNotificationsAudioGetRequestSchemaExecute = logsAgentsCompletionsRequestNotificationsAudioGetRequestSchemaExecute;
exports.logsAgentsCompletionsRequestNotificationsAudioGetRequestSchemaExecuteJq = logsAgentsCompletionsRequestNotificationsAudioGetRequestSchemaExecuteJq;
exports.logsAgentsCompletionsRequestNotificationsAudioGetResponseSchemaExecute = logsAgentsCompletionsRequestNotificationsAudioGetResponseSchemaExecute;
exports.logsAgentsCompletionsRequestNotificationsAudioGetResponseSchemaExecuteJq = logsAgentsCompletionsRequestNotificationsAudioGetResponseSchemaExecuteJq;
exports.logsAgentsCompletionsRequestNotificationsFileGetExecute = logsAgentsCompletionsRequestNotificationsFileGetExecute;
exports.logsAgentsCompletionsRequestNotificationsFileGetExecuteJq = logsAgentsCompletionsRequestNotificationsFileGetExecuteJq;
exports.logsAgentsCompletionsRequestNotificationsFileGetRequestSchemaExecute = logsAgentsCompletionsRequestNotificationsFileGetRequestSchemaExecute;
exports.logsAgentsCompletionsRequestNotificationsFileGetRequestSchemaExecuteJq = logsAgentsCompletionsRequestNotificationsFileGetRequestSchemaExecuteJq;
exports.logsAgentsCompletionsRequestNotificationsFileGetResponseSchemaExecute = logsAgentsCompletionsRequestNotificationsFileGetResponseSchemaExecute;
exports.logsAgentsCompletionsRequestNotificationsFileGetResponseSchemaExecuteJq = logsAgentsCompletionsRequestNotificationsFileGetResponseSchemaExecuteJq;
exports.logsAgentsCompletionsRequestNotificationsImageGetExecute = logsAgentsCompletionsRequestNotificationsImageGetExecute;
exports.logsAgentsCompletionsRequestNotificationsImageGetExecuteJq = logsAgentsCompletionsRequestNotificationsImageGetExecuteJq;
exports.logsAgentsCompletionsRequestNotificationsImageGetRequestSchemaExecute = logsAgentsCompletionsRequestNotificationsImageGetRequestSchemaExecute;
exports.logsAgentsCompletionsRequestNotificationsImageGetRequestSchemaExecuteJq = logsAgentsCompletionsRequestNotificationsImageGetRequestSchemaExecuteJq;
exports.logsAgentsCompletionsRequestNotificationsImageGetResponseSchemaExecute = logsAgentsCompletionsRequestNotificationsImageGetResponseSchemaExecute;
exports.logsAgentsCompletionsRequestNotificationsImageGetResponseSchemaExecuteJq = logsAgentsCompletionsRequestNotificationsImageGetResponseSchemaExecuteJq;
exports.logsAgentsCompletionsRequestNotificationsTextGetExecute = logsAgentsCompletionsRequestNotificationsTextGetExecute;
exports.logsAgentsCompletionsRequestNotificationsTextGetExecuteJq = logsAgentsCompletionsRequestNotificationsTextGetExecuteJq;
exports.logsAgentsCompletionsRequestNotificationsTextGetRequestSchemaExecute = logsAgentsCompletionsRequestNotificationsTextGetRequestSchemaExecute;
exports.logsAgentsCompletionsRequestNotificationsTextGetRequestSchemaExecuteJq = logsAgentsCompletionsRequestNotificationsTextGetRequestSchemaExecuteJq;
exports.logsAgentsCompletionsRequestNotificationsTextGetResponseSchemaExecute = logsAgentsCompletionsRequestNotificationsTextGetResponseSchemaExecute;
exports.logsAgentsCompletionsRequestNotificationsTextGetResponseSchemaExecuteJq = logsAgentsCompletionsRequestNotificationsTextGetResponseSchemaExecuteJq;
exports.logsAgentsCompletionsRequestNotificationsVideoGetExecute = logsAgentsCompletionsRequestNotificationsVideoGetExecute;
exports.logsAgentsCompletionsRequestNotificationsVideoGetExecuteJq = logsAgentsCompletionsRequestNotificationsVideoGetExecuteJq;
exports.logsAgentsCompletionsRequestNotificationsVideoGetRequestSchemaExecute = logsAgentsCompletionsRequestNotificationsVideoGetRequestSchemaExecute;
exports.logsAgentsCompletionsRequestNotificationsVideoGetRequestSchemaExecuteJq = logsAgentsCompletionsRequestNotificationsVideoGetRequestSchemaExecuteJq;
exports.logsAgentsCompletionsRequestNotificationsVideoGetResponseSchemaExecute = logsAgentsCompletionsRequestNotificationsVideoGetResponseSchemaExecute;
exports.logsAgentsCompletionsRequestNotificationsVideoGetResponseSchemaExecuteJq = logsAgentsCompletionsRequestNotificationsVideoGetResponseSchemaExecuteJq;
exports.logsAgentsCompletionsRequestSubscribeExecute = logsAgentsCompletionsRequestSubscribeExecute;
exports.logsAgentsCompletionsRequestSubscribeExecuteJq = logsAgentsCompletionsRequestSubscribeExecuteJq;
exports.logsAgentsCompletionsRequestSubscribeRequestSchemaExecute = logsAgentsCompletionsRequestSubscribeRequestSchemaExecute;
exports.logsAgentsCompletionsRequestSubscribeRequestSchemaExecuteJq = logsAgentsCompletionsRequestSubscribeRequestSchemaExecuteJq;
exports.logsAgentsCompletionsRequestSubscribeResponseSchemaExecute = logsAgentsCompletionsRequestSubscribeResponseSchemaExecute;
exports.logsAgentsCompletionsRequestSubscribeResponseSchemaExecuteJq = logsAgentsCompletionsRequestSubscribeResponseSchemaExecuteJq;
exports.logsAgentsCompletionsResponseClearExecute = logsAgentsCompletionsResponseClearExecute;
exports.logsAgentsCompletionsResponseClearExecuteJq = logsAgentsCompletionsResponseClearExecuteJq;
exports.logsAgentsCompletionsResponseClearRequestSchemaExecute = logsAgentsCompletionsResponseClearRequestSchemaExecute;
exports.logsAgentsCompletionsResponseClearRequestSchemaExecuteJq = logsAgentsCompletionsResponseClearRequestSchemaExecuteJq;
exports.logsAgentsCompletionsResponseClearResponseSchemaExecute = logsAgentsCompletionsResponseClearResponseSchemaExecute;
exports.logsAgentsCompletionsResponseClearResponseSchemaExecuteJq = logsAgentsCompletionsResponseClearResponseSchemaExecuteJq;
exports.logsAgentsCompletionsResponseContinuationsClearExecute = logsAgentsCompletionsResponseContinuationsClearExecute;
exports.logsAgentsCompletionsResponseContinuationsClearExecuteJq = logsAgentsCompletionsResponseContinuationsClearExecuteJq;
exports.logsAgentsCompletionsResponseContinuationsClearRequestSchemaExecute = logsAgentsCompletionsResponseContinuationsClearRequestSchemaExecute;
exports.logsAgentsCompletionsResponseContinuationsClearRequestSchemaExecuteJq = logsAgentsCompletionsResponseContinuationsClearRequestSchemaExecuteJq;
exports.logsAgentsCompletionsResponseContinuationsClearResponseSchemaExecute = logsAgentsCompletionsResponseContinuationsClearResponseSchemaExecute;
exports.logsAgentsCompletionsResponseContinuationsClearResponseSchemaExecuteJq = logsAgentsCompletionsResponseContinuationsClearResponseSchemaExecuteJq;
exports.logsAgentsCompletionsResponseContinuationsGetExecute = logsAgentsCompletionsResponseContinuationsGetExecute;
exports.logsAgentsCompletionsResponseContinuationsGetExecuteJq = logsAgentsCompletionsResponseContinuationsGetExecuteJq;
exports.logsAgentsCompletionsResponseContinuationsGetRequestSchemaExecute = logsAgentsCompletionsResponseContinuationsGetRequestSchemaExecute;
exports.logsAgentsCompletionsResponseContinuationsGetRequestSchemaExecuteJq = logsAgentsCompletionsResponseContinuationsGetRequestSchemaExecuteJq;
exports.logsAgentsCompletionsResponseContinuationsGetResponseSchemaExecute = logsAgentsCompletionsResponseContinuationsGetResponseSchemaExecute;
exports.logsAgentsCompletionsResponseContinuationsGetResponseSchemaExecuteJq = logsAgentsCompletionsResponseContinuationsGetResponseSchemaExecuteJq;
exports.logsAgentsCompletionsResponseContinuationsSubscribeExecute = logsAgentsCompletionsResponseContinuationsSubscribeExecute;
exports.logsAgentsCompletionsResponseContinuationsSubscribeExecuteJq = logsAgentsCompletionsResponseContinuationsSubscribeExecuteJq;
exports.logsAgentsCompletionsResponseContinuationsSubscribeRequestSchemaExecute = logsAgentsCompletionsResponseContinuationsSubscribeRequestSchemaExecute;
exports.logsAgentsCompletionsResponseContinuationsSubscribeRequestSchemaExecuteJq = logsAgentsCompletionsResponseContinuationsSubscribeRequestSchemaExecuteJq;
exports.logsAgentsCompletionsResponseContinuationsSubscribeResponseSchemaExecute = logsAgentsCompletionsResponseContinuationsSubscribeResponseSchemaExecute;
exports.logsAgentsCompletionsResponseContinuationsSubscribeResponseSchemaExecuteJq = logsAgentsCompletionsResponseContinuationsSubscribeResponseSchemaExecuteJq;
exports.logsAgentsCompletionsResponseGetExecute = logsAgentsCompletionsResponseGetExecute;
exports.logsAgentsCompletionsResponseGetExecuteJq = logsAgentsCompletionsResponseGetExecuteJq;
exports.logsAgentsCompletionsResponseGetRequestSchemaExecute = logsAgentsCompletionsResponseGetRequestSchemaExecute;
exports.logsAgentsCompletionsResponseGetRequestSchemaExecuteJq = logsAgentsCompletionsResponseGetRequestSchemaExecuteJq;
exports.logsAgentsCompletionsResponseGetResponseSchemaExecute = logsAgentsCompletionsResponseGetResponseSchemaExecute;
exports.logsAgentsCompletionsResponseGetResponseSchemaExecuteJq = logsAgentsCompletionsResponseGetResponseSchemaExecuteJq;
exports.logsAgentsCompletionsResponseListExecute = logsAgentsCompletionsResponseListExecute;
exports.logsAgentsCompletionsResponseListExecuteJq = logsAgentsCompletionsResponseListExecuteJq;
exports.logsAgentsCompletionsResponseListRequestSchemaExecute = logsAgentsCompletionsResponseListRequestSchemaExecute;
exports.logsAgentsCompletionsResponseListRequestSchemaExecuteJq = logsAgentsCompletionsResponseListRequestSchemaExecuteJq;
exports.logsAgentsCompletionsResponseListResponseSchemaExecute = logsAgentsCompletionsResponseListResponseSchemaExecute;
exports.logsAgentsCompletionsResponseListResponseSchemaExecuteJq = logsAgentsCompletionsResponseListResponseSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesAudioClearExecute = logsAgentsCompletionsResponseMessagesAudioClearExecute;
exports.logsAgentsCompletionsResponseMessagesAudioClearExecuteJq = logsAgentsCompletionsResponseMessagesAudioClearExecuteJq;
exports.logsAgentsCompletionsResponseMessagesAudioClearRequestSchemaExecute = logsAgentsCompletionsResponseMessagesAudioClearRequestSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesAudioClearRequestSchemaExecuteJq = logsAgentsCompletionsResponseMessagesAudioClearRequestSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesAudioClearResponseSchemaExecute = logsAgentsCompletionsResponseMessagesAudioClearResponseSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesAudioClearResponseSchemaExecuteJq = logsAgentsCompletionsResponseMessagesAudioClearResponseSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesAudioGetExecute = logsAgentsCompletionsResponseMessagesAudioGetExecute;
exports.logsAgentsCompletionsResponseMessagesAudioGetExecuteJq = logsAgentsCompletionsResponseMessagesAudioGetExecuteJq;
exports.logsAgentsCompletionsResponseMessagesAudioGetRequestSchemaExecute = logsAgentsCompletionsResponseMessagesAudioGetRequestSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesAudioGetRequestSchemaExecuteJq = logsAgentsCompletionsResponseMessagesAudioGetRequestSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesAudioGetResponseSchemaExecute = logsAgentsCompletionsResponseMessagesAudioGetResponseSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesAudioGetResponseSchemaExecuteJq = logsAgentsCompletionsResponseMessagesAudioGetResponseSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesAudioSubscribeExecute = logsAgentsCompletionsResponseMessagesAudioSubscribeExecute;
exports.logsAgentsCompletionsResponseMessagesAudioSubscribeExecuteJq = logsAgentsCompletionsResponseMessagesAudioSubscribeExecuteJq;
exports.logsAgentsCompletionsResponseMessagesAudioSubscribeRequestSchemaExecute = logsAgentsCompletionsResponseMessagesAudioSubscribeRequestSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesAudioSubscribeRequestSchemaExecuteJq = logsAgentsCompletionsResponseMessagesAudioSubscribeRequestSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesAudioSubscribeResponseSchemaExecute = logsAgentsCompletionsResponseMessagesAudioSubscribeResponseSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesAudioSubscribeResponseSchemaExecuteJq = logsAgentsCompletionsResponseMessagesAudioSubscribeResponseSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesClearExecute = logsAgentsCompletionsResponseMessagesClearExecute;
exports.logsAgentsCompletionsResponseMessagesClearExecuteJq = logsAgentsCompletionsResponseMessagesClearExecuteJq;
exports.logsAgentsCompletionsResponseMessagesClearRequestSchemaExecute = logsAgentsCompletionsResponseMessagesClearRequestSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesClearRequestSchemaExecuteJq = logsAgentsCompletionsResponseMessagesClearRequestSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesClearResponseSchemaExecute = logsAgentsCompletionsResponseMessagesClearResponseSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesClearResponseSchemaExecuteJq = logsAgentsCompletionsResponseMessagesClearResponseSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesFileClearExecute = logsAgentsCompletionsResponseMessagesFileClearExecute;
exports.logsAgentsCompletionsResponseMessagesFileClearExecuteJq = logsAgentsCompletionsResponseMessagesFileClearExecuteJq;
exports.logsAgentsCompletionsResponseMessagesFileClearRequestSchemaExecute = logsAgentsCompletionsResponseMessagesFileClearRequestSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesFileClearRequestSchemaExecuteJq = logsAgentsCompletionsResponseMessagesFileClearRequestSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesFileClearResponseSchemaExecute = logsAgentsCompletionsResponseMessagesFileClearResponseSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesFileClearResponseSchemaExecuteJq = logsAgentsCompletionsResponseMessagesFileClearResponseSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesFileGetExecute = logsAgentsCompletionsResponseMessagesFileGetExecute;
exports.logsAgentsCompletionsResponseMessagesFileGetExecuteJq = logsAgentsCompletionsResponseMessagesFileGetExecuteJq;
exports.logsAgentsCompletionsResponseMessagesFileGetRequestSchemaExecute = logsAgentsCompletionsResponseMessagesFileGetRequestSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesFileGetRequestSchemaExecuteJq = logsAgentsCompletionsResponseMessagesFileGetRequestSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesFileGetResponseSchemaExecute = logsAgentsCompletionsResponseMessagesFileGetResponseSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesFileGetResponseSchemaExecuteJq = logsAgentsCompletionsResponseMessagesFileGetResponseSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesFileSubscribeExecute = logsAgentsCompletionsResponseMessagesFileSubscribeExecute;
exports.logsAgentsCompletionsResponseMessagesFileSubscribeExecuteJq = logsAgentsCompletionsResponseMessagesFileSubscribeExecuteJq;
exports.logsAgentsCompletionsResponseMessagesFileSubscribeRequestSchemaExecute = logsAgentsCompletionsResponseMessagesFileSubscribeRequestSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesFileSubscribeRequestSchemaExecuteJq = logsAgentsCompletionsResponseMessagesFileSubscribeRequestSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesFileSubscribeResponseSchemaExecute = logsAgentsCompletionsResponseMessagesFileSubscribeResponseSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesFileSubscribeResponseSchemaExecuteJq = logsAgentsCompletionsResponseMessagesFileSubscribeResponseSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesGetExecute = logsAgentsCompletionsResponseMessagesGetExecute;
exports.logsAgentsCompletionsResponseMessagesGetExecuteJq = logsAgentsCompletionsResponseMessagesGetExecuteJq;
exports.logsAgentsCompletionsResponseMessagesGetRequestSchemaExecute = logsAgentsCompletionsResponseMessagesGetRequestSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesGetRequestSchemaExecuteJq = logsAgentsCompletionsResponseMessagesGetRequestSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesGetResponseSchemaExecute = logsAgentsCompletionsResponseMessagesGetResponseSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesGetResponseSchemaExecuteJq = logsAgentsCompletionsResponseMessagesGetResponseSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesImageClearExecute = logsAgentsCompletionsResponseMessagesImageClearExecute;
exports.logsAgentsCompletionsResponseMessagesImageClearExecuteJq = logsAgentsCompletionsResponseMessagesImageClearExecuteJq;
exports.logsAgentsCompletionsResponseMessagesImageClearRequestSchemaExecute = logsAgentsCompletionsResponseMessagesImageClearRequestSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesImageClearRequestSchemaExecuteJq = logsAgentsCompletionsResponseMessagesImageClearRequestSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesImageClearResponseSchemaExecute = logsAgentsCompletionsResponseMessagesImageClearResponseSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesImageClearResponseSchemaExecuteJq = logsAgentsCompletionsResponseMessagesImageClearResponseSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesImageGetExecute = logsAgentsCompletionsResponseMessagesImageGetExecute;
exports.logsAgentsCompletionsResponseMessagesImageGetExecuteJq = logsAgentsCompletionsResponseMessagesImageGetExecuteJq;
exports.logsAgentsCompletionsResponseMessagesImageGetRequestSchemaExecute = logsAgentsCompletionsResponseMessagesImageGetRequestSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesImageGetRequestSchemaExecuteJq = logsAgentsCompletionsResponseMessagesImageGetRequestSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesImageGetResponseSchemaExecute = logsAgentsCompletionsResponseMessagesImageGetResponseSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesImageGetResponseSchemaExecuteJq = logsAgentsCompletionsResponseMessagesImageGetResponseSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesImageSubscribeExecute = logsAgentsCompletionsResponseMessagesImageSubscribeExecute;
exports.logsAgentsCompletionsResponseMessagesImageSubscribeExecuteJq = logsAgentsCompletionsResponseMessagesImageSubscribeExecuteJq;
exports.logsAgentsCompletionsResponseMessagesImageSubscribeRequestSchemaExecute = logsAgentsCompletionsResponseMessagesImageSubscribeRequestSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesImageSubscribeRequestSchemaExecuteJq = logsAgentsCompletionsResponseMessagesImageSubscribeRequestSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesImageSubscribeResponseSchemaExecute = logsAgentsCompletionsResponseMessagesImageSubscribeResponseSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesImageSubscribeResponseSchemaExecuteJq = logsAgentsCompletionsResponseMessagesImageSubscribeResponseSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesLogprobsClearExecute = logsAgentsCompletionsResponseMessagesLogprobsClearExecute;
exports.logsAgentsCompletionsResponseMessagesLogprobsClearExecuteJq = logsAgentsCompletionsResponseMessagesLogprobsClearExecuteJq;
exports.logsAgentsCompletionsResponseMessagesLogprobsClearRequestSchemaExecute = logsAgentsCompletionsResponseMessagesLogprobsClearRequestSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesLogprobsClearRequestSchemaExecuteJq = logsAgentsCompletionsResponseMessagesLogprobsClearRequestSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesLogprobsClearResponseSchemaExecute = logsAgentsCompletionsResponseMessagesLogprobsClearResponseSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesLogprobsClearResponseSchemaExecuteJq = logsAgentsCompletionsResponseMessagesLogprobsClearResponseSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesLogprobsGetExecute = logsAgentsCompletionsResponseMessagesLogprobsGetExecute;
exports.logsAgentsCompletionsResponseMessagesLogprobsGetExecuteJq = logsAgentsCompletionsResponseMessagesLogprobsGetExecuteJq;
exports.logsAgentsCompletionsResponseMessagesLogprobsGetRequestSchemaExecute = logsAgentsCompletionsResponseMessagesLogprobsGetRequestSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesLogprobsGetRequestSchemaExecuteJq = logsAgentsCompletionsResponseMessagesLogprobsGetRequestSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesLogprobsGetResponseSchemaExecute = logsAgentsCompletionsResponseMessagesLogprobsGetResponseSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesLogprobsGetResponseSchemaExecuteJq = logsAgentsCompletionsResponseMessagesLogprobsGetResponseSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesLogprobsSubscribeExecute = logsAgentsCompletionsResponseMessagesLogprobsSubscribeExecute;
exports.logsAgentsCompletionsResponseMessagesLogprobsSubscribeExecuteJq = logsAgentsCompletionsResponseMessagesLogprobsSubscribeExecuteJq;
exports.logsAgentsCompletionsResponseMessagesLogprobsSubscribeRequestSchemaExecute = logsAgentsCompletionsResponseMessagesLogprobsSubscribeRequestSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesLogprobsSubscribeRequestSchemaExecuteJq = logsAgentsCompletionsResponseMessagesLogprobsSubscribeRequestSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesLogprobsSubscribeResponseSchemaExecute = logsAgentsCompletionsResponseMessagesLogprobsSubscribeResponseSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesLogprobsSubscribeResponseSchemaExecuteJq = logsAgentsCompletionsResponseMessagesLogprobsSubscribeResponseSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesReasoningClearExecute = logsAgentsCompletionsResponseMessagesReasoningClearExecute;
exports.logsAgentsCompletionsResponseMessagesReasoningClearExecuteJq = logsAgentsCompletionsResponseMessagesReasoningClearExecuteJq;
exports.logsAgentsCompletionsResponseMessagesReasoningClearRequestSchemaExecute = logsAgentsCompletionsResponseMessagesReasoningClearRequestSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesReasoningClearRequestSchemaExecuteJq = logsAgentsCompletionsResponseMessagesReasoningClearRequestSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesReasoningClearResponseSchemaExecute = logsAgentsCompletionsResponseMessagesReasoningClearResponseSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesReasoningClearResponseSchemaExecuteJq = logsAgentsCompletionsResponseMessagesReasoningClearResponseSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesReasoningGetExecute = logsAgentsCompletionsResponseMessagesReasoningGetExecute;
exports.logsAgentsCompletionsResponseMessagesReasoningGetExecuteJq = logsAgentsCompletionsResponseMessagesReasoningGetExecuteJq;
exports.logsAgentsCompletionsResponseMessagesReasoningGetRequestSchemaExecute = logsAgentsCompletionsResponseMessagesReasoningGetRequestSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesReasoningGetRequestSchemaExecuteJq = logsAgentsCompletionsResponseMessagesReasoningGetRequestSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesReasoningGetResponseSchemaExecute = logsAgentsCompletionsResponseMessagesReasoningGetResponseSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesReasoningGetResponseSchemaExecuteJq = logsAgentsCompletionsResponseMessagesReasoningGetResponseSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesReasoningSubscribeExecute = logsAgentsCompletionsResponseMessagesReasoningSubscribeExecute;
exports.logsAgentsCompletionsResponseMessagesReasoningSubscribeExecuteJq = logsAgentsCompletionsResponseMessagesReasoningSubscribeExecuteJq;
exports.logsAgentsCompletionsResponseMessagesReasoningSubscribeRequestSchemaExecute = logsAgentsCompletionsResponseMessagesReasoningSubscribeRequestSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesReasoningSubscribeRequestSchemaExecuteJq = logsAgentsCompletionsResponseMessagesReasoningSubscribeRequestSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesReasoningSubscribeResponseSchemaExecute = logsAgentsCompletionsResponseMessagesReasoningSubscribeResponseSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesReasoningSubscribeResponseSchemaExecuteJq = logsAgentsCompletionsResponseMessagesReasoningSubscribeResponseSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesRefusalClearExecute = logsAgentsCompletionsResponseMessagesRefusalClearExecute;
exports.logsAgentsCompletionsResponseMessagesRefusalClearExecuteJq = logsAgentsCompletionsResponseMessagesRefusalClearExecuteJq;
exports.logsAgentsCompletionsResponseMessagesRefusalClearRequestSchemaExecute = logsAgentsCompletionsResponseMessagesRefusalClearRequestSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesRefusalClearRequestSchemaExecuteJq = logsAgentsCompletionsResponseMessagesRefusalClearRequestSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesRefusalClearResponseSchemaExecute = logsAgentsCompletionsResponseMessagesRefusalClearResponseSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesRefusalClearResponseSchemaExecuteJq = logsAgentsCompletionsResponseMessagesRefusalClearResponseSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesRefusalGetExecute = logsAgentsCompletionsResponseMessagesRefusalGetExecute;
exports.logsAgentsCompletionsResponseMessagesRefusalGetExecuteJq = logsAgentsCompletionsResponseMessagesRefusalGetExecuteJq;
exports.logsAgentsCompletionsResponseMessagesRefusalGetRequestSchemaExecute = logsAgentsCompletionsResponseMessagesRefusalGetRequestSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesRefusalGetRequestSchemaExecuteJq = logsAgentsCompletionsResponseMessagesRefusalGetRequestSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesRefusalGetResponseSchemaExecute = logsAgentsCompletionsResponseMessagesRefusalGetResponseSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesRefusalGetResponseSchemaExecuteJq = logsAgentsCompletionsResponseMessagesRefusalGetResponseSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesRefusalSubscribeExecute = logsAgentsCompletionsResponseMessagesRefusalSubscribeExecute;
exports.logsAgentsCompletionsResponseMessagesRefusalSubscribeExecuteJq = logsAgentsCompletionsResponseMessagesRefusalSubscribeExecuteJq;
exports.logsAgentsCompletionsResponseMessagesRefusalSubscribeRequestSchemaExecute = logsAgentsCompletionsResponseMessagesRefusalSubscribeRequestSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesRefusalSubscribeRequestSchemaExecuteJq = logsAgentsCompletionsResponseMessagesRefusalSubscribeRequestSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesRefusalSubscribeResponseSchemaExecute = logsAgentsCompletionsResponseMessagesRefusalSubscribeResponseSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesRefusalSubscribeResponseSchemaExecuteJq = logsAgentsCompletionsResponseMessagesRefusalSubscribeResponseSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesSubscribeExecute = logsAgentsCompletionsResponseMessagesSubscribeExecute;
exports.logsAgentsCompletionsResponseMessagesSubscribeExecuteJq = logsAgentsCompletionsResponseMessagesSubscribeExecuteJq;
exports.logsAgentsCompletionsResponseMessagesSubscribeRequestSchemaExecute = logsAgentsCompletionsResponseMessagesSubscribeRequestSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesSubscribeRequestSchemaExecuteJq = logsAgentsCompletionsResponseMessagesSubscribeRequestSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesSubscribeResponseSchemaExecute = logsAgentsCompletionsResponseMessagesSubscribeResponseSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesSubscribeResponseSchemaExecuteJq = logsAgentsCompletionsResponseMessagesSubscribeResponseSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesTextGetExecute = logsAgentsCompletionsResponseMessagesTextGetExecute;
exports.logsAgentsCompletionsResponseMessagesTextGetExecuteJq = logsAgentsCompletionsResponseMessagesTextGetExecuteJq;
exports.logsAgentsCompletionsResponseMessagesTextGetRequestSchemaExecute = logsAgentsCompletionsResponseMessagesTextGetRequestSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesTextGetRequestSchemaExecuteJq = logsAgentsCompletionsResponseMessagesTextGetRequestSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesTextGetResponseSchemaExecute = logsAgentsCompletionsResponseMessagesTextGetResponseSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesTextGetResponseSchemaExecuteJq = logsAgentsCompletionsResponseMessagesTextGetResponseSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesToolAudioGetExecute = logsAgentsCompletionsResponseMessagesToolAudioGetExecute;
exports.logsAgentsCompletionsResponseMessagesToolAudioGetExecuteJq = logsAgentsCompletionsResponseMessagesToolAudioGetExecuteJq;
exports.logsAgentsCompletionsResponseMessagesToolAudioGetRequestSchemaExecute = logsAgentsCompletionsResponseMessagesToolAudioGetRequestSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesToolAudioGetRequestSchemaExecuteJq = logsAgentsCompletionsResponseMessagesToolAudioGetRequestSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesToolAudioGetResponseSchemaExecute = logsAgentsCompletionsResponseMessagesToolAudioGetResponseSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesToolAudioGetResponseSchemaExecuteJq = logsAgentsCompletionsResponseMessagesToolAudioGetResponseSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesToolCallsClearExecute = logsAgentsCompletionsResponseMessagesToolCallsClearExecute;
exports.logsAgentsCompletionsResponseMessagesToolCallsClearExecuteJq = logsAgentsCompletionsResponseMessagesToolCallsClearExecuteJq;
exports.logsAgentsCompletionsResponseMessagesToolCallsClearRequestSchemaExecute = logsAgentsCompletionsResponseMessagesToolCallsClearRequestSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesToolCallsClearRequestSchemaExecuteJq = logsAgentsCompletionsResponseMessagesToolCallsClearRequestSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesToolCallsClearResponseSchemaExecute = logsAgentsCompletionsResponseMessagesToolCallsClearResponseSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesToolCallsClearResponseSchemaExecuteJq = logsAgentsCompletionsResponseMessagesToolCallsClearResponseSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesToolCallsGetExecute = logsAgentsCompletionsResponseMessagesToolCallsGetExecute;
exports.logsAgentsCompletionsResponseMessagesToolCallsGetExecuteJq = logsAgentsCompletionsResponseMessagesToolCallsGetExecuteJq;
exports.logsAgentsCompletionsResponseMessagesToolCallsGetRequestSchemaExecute = logsAgentsCompletionsResponseMessagesToolCallsGetRequestSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesToolCallsGetRequestSchemaExecuteJq = logsAgentsCompletionsResponseMessagesToolCallsGetRequestSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesToolCallsGetResponseSchemaExecute = logsAgentsCompletionsResponseMessagesToolCallsGetResponseSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesToolCallsGetResponseSchemaExecuteJq = logsAgentsCompletionsResponseMessagesToolCallsGetResponseSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesToolCallsSubscribeExecute = logsAgentsCompletionsResponseMessagesToolCallsSubscribeExecute;
exports.logsAgentsCompletionsResponseMessagesToolCallsSubscribeExecuteJq = logsAgentsCompletionsResponseMessagesToolCallsSubscribeExecuteJq;
exports.logsAgentsCompletionsResponseMessagesToolCallsSubscribeRequestSchemaExecute = logsAgentsCompletionsResponseMessagesToolCallsSubscribeRequestSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesToolCallsSubscribeRequestSchemaExecuteJq = logsAgentsCompletionsResponseMessagesToolCallsSubscribeRequestSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesToolCallsSubscribeResponseSchemaExecute = logsAgentsCompletionsResponseMessagesToolCallsSubscribeResponseSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesToolCallsSubscribeResponseSchemaExecuteJq = logsAgentsCompletionsResponseMessagesToolCallsSubscribeResponseSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesToolFileGetExecute = logsAgentsCompletionsResponseMessagesToolFileGetExecute;
exports.logsAgentsCompletionsResponseMessagesToolFileGetExecuteJq = logsAgentsCompletionsResponseMessagesToolFileGetExecuteJq;
exports.logsAgentsCompletionsResponseMessagesToolFileGetRequestSchemaExecute = logsAgentsCompletionsResponseMessagesToolFileGetRequestSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesToolFileGetRequestSchemaExecuteJq = logsAgentsCompletionsResponseMessagesToolFileGetRequestSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesToolFileGetResponseSchemaExecute = logsAgentsCompletionsResponseMessagesToolFileGetResponseSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesToolFileGetResponseSchemaExecuteJq = logsAgentsCompletionsResponseMessagesToolFileGetResponseSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesToolImageGetExecute = logsAgentsCompletionsResponseMessagesToolImageGetExecute;
exports.logsAgentsCompletionsResponseMessagesToolImageGetExecuteJq = logsAgentsCompletionsResponseMessagesToolImageGetExecuteJq;
exports.logsAgentsCompletionsResponseMessagesToolImageGetRequestSchemaExecute = logsAgentsCompletionsResponseMessagesToolImageGetRequestSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesToolImageGetRequestSchemaExecuteJq = logsAgentsCompletionsResponseMessagesToolImageGetRequestSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesToolImageGetResponseSchemaExecute = logsAgentsCompletionsResponseMessagesToolImageGetResponseSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesToolImageGetResponseSchemaExecuteJq = logsAgentsCompletionsResponseMessagesToolImageGetResponseSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesToolTextGetExecute = logsAgentsCompletionsResponseMessagesToolTextGetExecute;
exports.logsAgentsCompletionsResponseMessagesToolTextGetExecuteJq = logsAgentsCompletionsResponseMessagesToolTextGetExecuteJq;
exports.logsAgentsCompletionsResponseMessagesToolTextGetRequestSchemaExecute = logsAgentsCompletionsResponseMessagesToolTextGetRequestSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesToolTextGetRequestSchemaExecuteJq = logsAgentsCompletionsResponseMessagesToolTextGetRequestSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesToolTextGetResponseSchemaExecute = logsAgentsCompletionsResponseMessagesToolTextGetResponseSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesToolTextGetResponseSchemaExecuteJq = logsAgentsCompletionsResponseMessagesToolTextGetResponseSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesToolVideoGetExecute = logsAgentsCompletionsResponseMessagesToolVideoGetExecute;
exports.logsAgentsCompletionsResponseMessagesToolVideoGetExecuteJq = logsAgentsCompletionsResponseMessagesToolVideoGetExecuteJq;
exports.logsAgentsCompletionsResponseMessagesToolVideoGetRequestSchemaExecute = logsAgentsCompletionsResponseMessagesToolVideoGetRequestSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesToolVideoGetRequestSchemaExecuteJq = logsAgentsCompletionsResponseMessagesToolVideoGetRequestSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesToolVideoGetResponseSchemaExecute = logsAgentsCompletionsResponseMessagesToolVideoGetResponseSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesToolVideoGetResponseSchemaExecuteJq = logsAgentsCompletionsResponseMessagesToolVideoGetResponseSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesVideoClearExecute = logsAgentsCompletionsResponseMessagesVideoClearExecute;
exports.logsAgentsCompletionsResponseMessagesVideoClearExecuteJq = logsAgentsCompletionsResponseMessagesVideoClearExecuteJq;
exports.logsAgentsCompletionsResponseMessagesVideoClearRequestSchemaExecute = logsAgentsCompletionsResponseMessagesVideoClearRequestSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesVideoClearRequestSchemaExecuteJq = logsAgentsCompletionsResponseMessagesVideoClearRequestSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesVideoClearResponseSchemaExecute = logsAgentsCompletionsResponseMessagesVideoClearResponseSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesVideoClearResponseSchemaExecuteJq = logsAgentsCompletionsResponseMessagesVideoClearResponseSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesVideoGetExecute = logsAgentsCompletionsResponseMessagesVideoGetExecute;
exports.logsAgentsCompletionsResponseMessagesVideoGetExecuteJq = logsAgentsCompletionsResponseMessagesVideoGetExecuteJq;
exports.logsAgentsCompletionsResponseMessagesVideoGetRequestSchemaExecute = logsAgentsCompletionsResponseMessagesVideoGetRequestSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesVideoGetRequestSchemaExecuteJq = logsAgentsCompletionsResponseMessagesVideoGetRequestSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesVideoGetResponseSchemaExecute = logsAgentsCompletionsResponseMessagesVideoGetResponseSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesVideoGetResponseSchemaExecuteJq = logsAgentsCompletionsResponseMessagesVideoGetResponseSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesVideoSubscribeExecute = logsAgentsCompletionsResponseMessagesVideoSubscribeExecute;
exports.logsAgentsCompletionsResponseMessagesVideoSubscribeExecuteJq = logsAgentsCompletionsResponseMessagesVideoSubscribeExecuteJq;
exports.logsAgentsCompletionsResponseMessagesVideoSubscribeRequestSchemaExecute = logsAgentsCompletionsResponseMessagesVideoSubscribeRequestSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesVideoSubscribeRequestSchemaExecuteJq = logsAgentsCompletionsResponseMessagesVideoSubscribeRequestSchemaExecuteJq;
exports.logsAgentsCompletionsResponseMessagesVideoSubscribeResponseSchemaExecute = logsAgentsCompletionsResponseMessagesVideoSubscribeResponseSchemaExecute;
exports.logsAgentsCompletionsResponseMessagesVideoSubscribeResponseSchemaExecuteJq = logsAgentsCompletionsResponseMessagesVideoSubscribeResponseSchemaExecuteJq;
exports.logsAgentsCompletionsResponseSubscribeExecute = logsAgentsCompletionsResponseSubscribeExecute;
exports.logsAgentsCompletionsResponseSubscribeExecuteJq = logsAgentsCompletionsResponseSubscribeExecuteJq;
exports.logsAgentsCompletionsResponseSubscribeRequestSchemaExecute = logsAgentsCompletionsResponseSubscribeRequestSchemaExecute;
exports.logsAgentsCompletionsResponseSubscribeRequestSchemaExecuteJq = logsAgentsCompletionsResponseSubscribeRequestSchemaExecuteJq;
exports.logsAgentsCompletionsResponseSubscribeResponseSchemaExecute = logsAgentsCompletionsResponseSubscribeResponseSchemaExecute;
exports.logsAgentsCompletionsResponseSubscribeResponseSchemaExecuteJq = logsAgentsCompletionsResponseSubscribeResponseSchemaExecuteJq;
exports.logsClearExecute = logsClearExecute;
exports.logsClearExecuteJq = logsClearExecuteJq;
exports.logsClearRequestSchemaExecute = logsClearRequestSchemaExecute;
exports.logsClearRequestSchemaExecuteJq = logsClearRequestSchemaExecuteJq;
exports.logsClearResponseSchemaExecute = logsClearResponseSchemaExecute;
exports.logsClearResponseSchemaExecuteJq = logsClearResponseSchemaExecuteJq;
exports.logsFunctionsExecutionsRequestGetExecute = logsFunctionsExecutionsRequestGetExecute;
exports.logsFunctionsExecutionsRequestGetExecuteJq = logsFunctionsExecutionsRequestGetExecuteJq;
exports.logsFunctionsExecutionsRequestGetRequestSchemaExecute = logsFunctionsExecutionsRequestGetRequestSchemaExecute;
exports.logsFunctionsExecutionsRequestGetRequestSchemaExecuteJq = logsFunctionsExecutionsRequestGetRequestSchemaExecuteJq;
exports.logsFunctionsExecutionsRequestGetResponseSchemaExecute = logsFunctionsExecutionsRequestGetResponseSchemaExecute;
exports.logsFunctionsExecutionsRequestGetResponseSchemaExecuteJq = logsFunctionsExecutionsRequestGetResponseSchemaExecuteJq;
exports.logsFunctionsExecutionsRequestSubscribeExecute = logsFunctionsExecutionsRequestSubscribeExecute;
exports.logsFunctionsExecutionsRequestSubscribeExecuteJq = logsFunctionsExecutionsRequestSubscribeExecuteJq;
exports.logsFunctionsExecutionsRequestSubscribeRequestSchemaExecute = logsFunctionsExecutionsRequestSubscribeRequestSchemaExecute;
exports.logsFunctionsExecutionsRequestSubscribeRequestSchemaExecuteJq = logsFunctionsExecutionsRequestSubscribeRequestSchemaExecuteJq;
exports.logsFunctionsExecutionsRequestSubscribeResponseSchemaExecute = logsFunctionsExecutionsRequestSubscribeResponseSchemaExecute;
exports.logsFunctionsExecutionsRequestSubscribeResponseSchemaExecuteJq = logsFunctionsExecutionsRequestSubscribeResponseSchemaExecuteJq;
exports.logsFunctionsExecutionsResponseClearExecute = logsFunctionsExecutionsResponseClearExecute;
exports.logsFunctionsExecutionsResponseClearExecuteJq = logsFunctionsExecutionsResponseClearExecuteJq;
exports.logsFunctionsExecutionsResponseClearRequestSchemaExecute = logsFunctionsExecutionsResponseClearRequestSchemaExecute;
exports.logsFunctionsExecutionsResponseClearRequestSchemaExecuteJq = logsFunctionsExecutionsResponseClearRequestSchemaExecuteJq;
exports.logsFunctionsExecutionsResponseClearResponseSchemaExecute = logsFunctionsExecutionsResponseClearResponseSchemaExecute;
exports.logsFunctionsExecutionsResponseClearResponseSchemaExecuteJq = logsFunctionsExecutionsResponseClearResponseSchemaExecuteJq;
exports.logsFunctionsExecutionsResponseGetExecute = logsFunctionsExecutionsResponseGetExecute;
exports.logsFunctionsExecutionsResponseGetExecuteJq = logsFunctionsExecutionsResponseGetExecuteJq;
exports.logsFunctionsExecutionsResponseGetRequestSchemaExecute = logsFunctionsExecutionsResponseGetRequestSchemaExecute;
exports.logsFunctionsExecutionsResponseGetRequestSchemaExecuteJq = logsFunctionsExecutionsResponseGetRequestSchemaExecuteJq;
exports.logsFunctionsExecutionsResponseGetResponseSchemaExecute = logsFunctionsExecutionsResponseGetResponseSchemaExecute;
exports.logsFunctionsExecutionsResponseGetResponseSchemaExecuteJq = logsFunctionsExecutionsResponseGetResponseSchemaExecuteJq;
exports.logsFunctionsExecutionsResponseListExecute = logsFunctionsExecutionsResponseListExecute;
exports.logsFunctionsExecutionsResponseListExecuteJq = logsFunctionsExecutionsResponseListExecuteJq;
exports.logsFunctionsExecutionsResponseListRequestSchemaExecute = logsFunctionsExecutionsResponseListRequestSchemaExecute;
exports.logsFunctionsExecutionsResponseListRequestSchemaExecuteJq = logsFunctionsExecutionsResponseListRequestSchemaExecuteJq;
exports.logsFunctionsExecutionsResponseListResponseSchemaExecute = logsFunctionsExecutionsResponseListResponseSchemaExecute;
exports.logsFunctionsExecutionsResponseListResponseSchemaExecuteJq = logsFunctionsExecutionsResponseListResponseSchemaExecuteJq;
exports.logsFunctionsExecutionsResponseRetryTokensClearExecute = logsFunctionsExecutionsResponseRetryTokensClearExecute;
exports.logsFunctionsExecutionsResponseRetryTokensClearExecuteJq = logsFunctionsExecutionsResponseRetryTokensClearExecuteJq;
exports.logsFunctionsExecutionsResponseRetryTokensClearRequestSchemaExecute = logsFunctionsExecutionsResponseRetryTokensClearRequestSchemaExecute;
exports.logsFunctionsExecutionsResponseRetryTokensClearRequestSchemaExecuteJq = logsFunctionsExecutionsResponseRetryTokensClearRequestSchemaExecuteJq;
exports.logsFunctionsExecutionsResponseRetryTokensClearResponseSchemaExecute = logsFunctionsExecutionsResponseRetryTokensClearResponseSchemaExecute;
exports.logsFunctionsExecutionsResponseRetryTokensClearResponseSchemaExecuteJq = logsFunctionsExecutionsResponseRetryTokensClearResponseSchemaExecuteJq;
exports.logsFunctionsExecutionsResponseRetryTokensGetExecute = logsFunctionsExecutionsResponseRetryTokensGetExecute;
exports.logsFunctionsExecutionsResponseRetryTokensGetExecuteJq = logsFunctionsExecutionsResponseRetryTokensGetExecuteJq;
exports.logsFunctionsExecutionsResponseRetryTokensGetRequestSchemaExecute = logsFunctionsExecutionsResponseRetryTokensGetRequestSchemaExecute;
exports.logsFunctionsExecutionsResponseRetryTokensGetRequestSchemaExecuteJq = logsFunctionsExecutionsResponseRetryTokensGetRequestSchemaExecuteJq;
exports.logsFunctionsExecutionsResponseRetryTokensGetResponseSchemaExecute = logsFunctionsExecutionsResponseRetryTokensGetResponseSchemaExecute;
exports.logsFunctionsExecutionsResponseRetryTokensGetResponseSchemaExecuteJq = logsFunctionsExecutionsResponseRetryTokensGetResponseSchemaExecuteJq;
exports.logsFunctionsExecutionsResponseRetryTokensSubscribeExecute = logsFunctionsExecutionsResponseRetryTokensSubscribeExecute;
exports.logsFunctionsExecutionsResponseRetryTokensSubscribeExecuteJq = logsFunctionsExecutionsResponseRetryTokensSubscribeExecuteJq;
exports.logsFunctionsExecutionsResponseRetryTokensSubscribeRequestSchemaExecute = logsFunctionsExecutionsResponseRetryTokensSubscribeRequestSchemaExecute;
exports.logsFunctionsExecutionsResponseRetryTokensSubscribeRequestSchemaExecuteJq = logsFunctionsExecutionsResponseRetryTokensSubscribeRequestSchemaExecuteJq;
exports.logsFunctionsExecutionsResponseRetryTokensSubscribeResponseSchemaExecute = logsFunctionsExecutionsResponseRetryTokensSubscribeResponseSchemaExecute;
exports.logsFunctionsExecutionsResponseRetryTokensSubscribeResponseSchemaExecuteJq = logsFunctionsExecutionsResponseRetryTokensSubscribeResponseSchemaExecuteJq;
exports.logsFunctionsExecutionsResponseSubscribeExecute = logsFunctionsExecutionsResponseSubscribeExecute;
exports.logsFunctionsExecutionsResponseSubscribeExecuteJq = logsFunctionsExecutionsResponseSubscribeExecuteJq;
exports.logsFunctionsExecutionsResponseSubscribeRequestSchemaExecute = logsFunctionsExecutionsResponseSubscribeRequestSchemaExecute;
exports.logsFunctionsExecutionsResponseSubscribeRequestSchemaExecuteJq = logsFunctionsExecutionsResponseSubscribeRequestSchemaExecuteJq;
exports.logsFunctionsExecutionsResponseSubscribeResponseSchemaExecute = logsFunctionsExecutionsResponseSubscribeResponseSchemaExecute;
exports.logsFunctionsExecutionsResponseSubscribeResponseSchemaExecuteJq = logsFunctionsExecutionsResponseSubscribeResponseSchemaExecuteJq;
exports.logsFunctionsInventionsRecursiveRequestGetExecute = logsFunctionsInventionsRecursiveRequestGetExecute;
exports.logsFunctionsInventionsRecursiveRequestGetExecuteJq = logsFunctionsInventionsRecursiveRequestGetExecuteJq;
exports.logsFunctionsInventionsRecursiveRequestGetRequestSchemaExecute = logsFunctionsInventionsRecursiveRequestGetRequestSchemaExecute;
exports.logsFunctionsInventionsRecursiveRequestGetRequestSchemaExecuteJq = logsFunctionsInventionsRecursiveRequestGetRequestSchemaExecuteJq;
exports.logsFunctionsInventionsRecursiveRequestGetResponseSchemaExecute = logsFunctionsInventionsRecursiveRequestGetResponseSchemaExecute;
exports.logsFunctionsInventionsRecursiveRequestGetResponseSchemaExecuteJq = logsFunctionsInventionsRecursiveRequestGetResponseSchemaExecuteJq;
exports.logsFunctionsInventionsRecursiveRequestSubscribeExecute = logsFunctionsInventionsRecursiveRequestSubscribeExecute;
exports.logsFunctionsInventionsRecursiveRequestSubscribeExecuteJq = logsFunctionsInventionsRecursiveRequestSubscribeExecuteJq;
exports.logsFunctionsInventionsRecursiveRequestSubscribeRequestSchemaExecute = logsFunctionsInventionsRecursiveRequestSubscribeRequestSchemaExecute;
exports.logsFunctionsInventionsRecursiveRequestSubscribeRequestSchemaExecuteJq = logsFunctionsInventionsRecursiveRequestSubscribeRequestSchemaExecuteJq;
exports.logsFunctionsInventionsRecursiveRequestSubscribeResponseSchemaExecute = logsFunctionsInventionsRecursiveRequestSubscribeResponseSchemaExecute;
exports.logsFunctionsInventionsRecursiveRequestSubscribeResponseSchemaExecuteJq = logsFunctionsInventionsRecursiveRequestSubscribeResponseSchemaExecuteJq;
exports.logsFunctionsInventionsRecursiveResponseClearExecute = logsFunctionsInventionsRecursiveResponseClearExecute;
exports.logsFunctionsInventionsRecursiveResponseClearExecuteJq = logsFunctionsInventionsRecursiveResponseClearExecuteJq;
exports.logsFunctionsInventionsRecursiveResponseClearRequestSchemaExecute = logsFunctionsInventionsRecursiveResponseClearRequestSchemaExecute;
exports.logsFunctionsInventionsRecursiveResponseClearRequestSchemaExecuteJq = logsFunctionsInventionsRecursiveResponseClearRequestSchemaExecuteJq;
exports.logsFunctionsInventionsRecursiveResponseClearResponseSchemaExecute = logsFunctionsInventionsRecursiveResponseClearResponseSchemaExecute;
exports.logsFunctionsInventionsRecursiveResponseClearResponseSchemaExecuteJq = logsFunctionsInventionsRecursiveResponseClearResponseSchemaExecuteJq;
exports.logsFunctionsInventionsRecursiveResponseGetExecute = logsFunctionsInventionsRecursiveResponseGetExecute;
exports.logsFunctionsInventionsRecursiveResponseGetExecuteJq = logsFunctionsInventionsRecursiveResponseGetExecuteJq;
exports.logsFunctionsInventionsRecursiveResponseGetRequestSchemaExecute = logsFunctionsInventionsRecursiveResponseGetRequestSchemaExecute;
exports.logsFunctionsInventionsRecursiveResponseGetRequestSchemaExecuteJq = logsFunctionsInventionsRecursiveResponseGetRequestSchemaExecuteJq;
exports.logsFunctionsInventionsRecursiveResponseGetResponseSchemaExecute = logsFunctionsInventionsRecursiveResponseGetResponseSchemaExecute;
exports.logsFunctionsInventionsRecursiveResponseGetResponseSchemaExecuteJq = logsFunctionsInventionsRecursiveResponseGetResponseSchemaExecuteJq;
exports.logsFunctionsInventionsRecursiveResponseListExecute = logsFunctionsInventionsRecursiveResponseListExecute;
exports.logsFunctionsInventionsRecursiveResponseListExecuteJq = logsFunctionsInventionsRecursiveResponseListExecuteJq;
exports.logsFunctionsInventionsRecursiveResponseListRequestSchemaExecute = logsFunctionsInventionsRecursiveResponseListRequestSchemaExecute;
exports.logsFunctionsInventionsRecursiveResponseListRequestSchemaExecuteJq = logsFunctionsInventionsRecursiveResponseListRequestSchemaExecuteJq;
exports.logsFunctionsInventionsRecursiveResponseListResponseSchemaExecute = logsFunctionsInventionsRecursiveResponseListResponseSchemaExecute;
exports.logsFunctionsInventionsRecursiveResponseListResponseSchemaExecuteJq = logsFunctionsInventionsRecursiveResponseListResponseSchemaExecuteJq;
exports.logsFunctionsInventionsRecursiveResponseSubscribeExecute = logsFunctionsInventionsRecursiveResponseSubscribeExecute;
exports.logsFunctionsInventionsRecursiveResponseSubscribeExecuteJq = logsFunctionsInventionsRecursiveResponseSubscribeExecuteJq;
exports.logsFunctionsInventionsRecursiveResponseSubscribeRequestSchemaExecute = logsFunctionsInventionsRecursiveResponseSubscribeRequestSchemaExecute;
exports.logsFunctionsInventionsRecursiveResponseSubscribeRequestSchemaExecuteJq = logsFunctionsInventionsRecursiveResponseSubscribeRequestSchemaExecuteJq;
exports.logsFunctionsInventionsRecursiveResponseSubscribeResponseSchemaExecute = logsFunctionsInventionsRecursiveResponseSubscribeResponseSchemaExecute;
exports.logsFunctionsInventionsRecursiveResponseSubscribeResponseSchemaExecuteJq = logsFunctionsInventionsRecursiveResponseSubscribeResponseSchemaExecuteJq;
exports.logsFunctionsInventionsRequestGetExecute = logsFunctionsInventionsRequestGetExecute;
exports.logsFunctionsInventionsRequestGetExecuteJq = logsFunctionsInventionsRequestGetExecuteJq;
exports.logsFunctionsInventionsRequestGetRequestSchemaExecute = logsFunctionsInventionsRequestGetRequestSchemaExecute;
exports.logsFunctionsInventionsRequestGetRequestSchemaExecuteJq = logsFunctionsInventionsRequestGetRequestSchemaExecuteJq;
exports.logsFunctionsInventionsRequestGetResponseSchemaExecute = logsFunctionsInventionsRequestGetResponseSchemaExecute;
exports.logsFunctionsInventionsRequestGetResponseSchemaExecuteJq = logsFunctionsInventionsRequestGetResponseSchemaExecuteJq;
exports.logsFunctionsInventionsRequestSubscribeExecute = logsFunctionsInventionsRequestSubscribeExecute;
exports.logsFunctionsInventionsRequestSubscribeExecuteJq = logsFunctionsInventionsRequestSubscribeExecuteJq;
exports.logsFunctionsInventionsRequestSubscribeRequestSchemaExecute = logsFunctionsInventionsRequestSubscribeRequestSchemaExecute;
exports.logsFunctionsInventionsRequestSubscribeRequestSchemaExecuteJq = logsFunctionsInventionsRequestSubscribeRequestSchemaExecuteJq;
exports.logsFunctionsInventionsRequestSubscribeResponseSchemaExecute = logsFunctionsInventionsRequestSubscribeResponseSchemaExecute;
exports.logsFunctionsInventionsRequestSubscribeResponseSchemaExecuteJq = logsFunctionsInventionsRequestSubscribeResponseSchemaExecuteJq;
exports.logsFunctionsInventionsResponseClearExecute = logsFunctionsInventionsResponseClearExecute;
exports.logsFunctionsInventionsResponseClearExecuteJq = logsFunctionsInventionsResponseClearExecuteJq;
exports.logsFunctionsInventionsResponseClearRequestSchemaExecute = logsFunctionsInventionsResponseClearRequestSchemaExecute;
exports.logsFunctionsInventionsResponseClearRequestSchemaExecuteJq = logsFunctionsInventionsResponseClearRequestSchemaExecuteJq;
exports.logsFunctionsInventionsResponseClearResponseSchemaExecute = logsFunctionsInventionsResponseClearResponseSchemaExecute;
exports.logsFunctionsInventionsResponseClearResponseSchemaExecuteJq = logsFunctionsInventionsResponseClearResponseSchemaExecuteJq;
exports.logsFunctionsInventionsResponseGetExecute = logsFunctionsInventionsResponseGetExecute;
exports.logsFunctionsInventionsResponseGetExecuteJq = logsFunctionsInventionsResponseGetExecuteJq;
exports.logsFunctionsInventionsResponseGetRequestSchemaExecute = logsFunctionsInventionsResponseGetRequestSchemaExecute;
exports.logsFunctionsInventionsResponseGetRequestSchemaExecuteJq = logsFunctionsInventionsResponseGetRequestSchemaExecuteJq;
exports.logsFunctionsInventionsResponseGetResponseSchemaExecute = logsFunctionsInventionsResponseGetResponseSchemaExecute;
exports.logsFunctionsInventionsResponseGetResponseSchemaExecuteJq = logsFunctionsInventionsResponseGetResponseSchemaExecuteJq;
exports.logsFunctionsInventionsResponseListExecute = logsFunctionsInventionsResponseListExecute;
exports.logsFunctionsInventionsResponseListExecuteJq = logsFunctionsInventionsResponseListExecuteJq;
exports.logsFunctionsInventionsResponseListRequestSchemaExecute = logsFunctionsInventionsResponseListRequestSchemaExecute;
exports.logsFunctionsInventionsResponseListRequestSchemaExecuteJq = logsFunctionsInventionsResponseListRequestSchemaExecuteJq;
exports.logsFunctionsInventionsResponseListResponseSchemaExecute = logsFunctionsInventionsResponseListResponseSchemaExecute;
exports.logsFunctionsInventionsResponseListResponseSchemaExecuteJq = logsFunctionsInventionsResponseListResponseSchemaExecuteJq;
exports.logsFunctionsInventionsResponseSubscribeExecute = logsFunctionsInventionsResponseSubscribeExecute;
exports.logsFunctionsInventionsResponseSubscribeExecuteJq = logsFunctionsInventionsResponseSubscribeExecuteJq;
exports.logsFunctionsInventionsResponseSubscribeRequestSchemaExecute = logsFunctionsInventionsResponseSubscribeRequestSchemaExecute;
exports.logsFunctionsInventionsResponseSubscribeRequestSchemaExecuteJq = logsFunctionsInventionsResponseSubscribeRequestSchemaExecuteJq;
exports.logsFunctionsInventionsResponseSubscribeResponseSchemaExecute = logsFunctionsInventionsResponseSubscribeResponseSchemaExecute;
exports.logsFunctionsInventionsResponseSubscribeResponseSchemaExecuteJq = logsFunctionsInventionsResponseSubscribeResponseSchemaExecuteJq;
exports.logsVectorCompletionsRequestGetExecute = logsVectorCompletionsRequestGetExecute;
exports.logsVectorCompletionsRequestGetExecuteJq = logsVectorCompletionsRequestGetExecuteJq;
exports.logsVectorCompletionsRequestGetRequestSchemaExecute = logsVectorCompletionsRequestGetRequestSchemaExecute;
exports.logsVectorCompletionsRequestGetRequestSchemaExecuteJq = logsVectorCompletionsRequestGetRequestSchemaExecuteJq;
exports.logsVectorCompletionsRequestGetResponseSchemaExecute = logsVectorCompletionsRequestGetResponseSchemaExecute;
exports.logsVectorCompletionsRequestGetResponseSchemaExecuteJq = logsVectorCompletionsRequestGetResponseSchemaExecuteJq;
exports.logsVectorCompletionsRequestSubscribeExecute = logsVectorCompletionsRequestSubscribeExecute;
exports.logsVectorCompletionsRequestSubscribeExecuteJq = logsVectorCompletionsRequestSubscribeExecuteJq;
exports.logsVectorCompletionsRequestSubscribeRequestSchemaExecute = logsVectorCompletionsRequestSubscribeRequestSchemaExecute;
exports.logsVectorCompletionsRequestSubscribeRequestSchemaExecuteJq = logsVectorCompletionsRequestSubscribeRequestSchemaExecuteJq;
exports.logsVectorCompletionsRequestSubscribeResponseSchemaExecute = logsVectorCompletionsRequestSubscribeResponseSchemaExecute;
exports.logsVectorCompletionsRequestSubscribeResponseSchemaExecuteJq = logsVectorCompletionsRequestSubscribeResponseSchemaExecuteJq;
exports.logsVectorCompletionsResponseClearExecute = logsVectorCompletionsResponseClearExecute;
exports.logsVectorCompletionsResponseClearExecuteJq = logsVectorCompletionsResponseClearExecuteJq;
exports.logsVectorCompletionsResponseClearRequestSchemaExecute = logsVectorCompletionsResponseClearRequestSchemaExecute;
exports.logsVectorCompletionsResponseClearRequestSchemaExecuteJq = logsVectorCompletionsResponseClearRequestSchemaExecuteJq;
exports.logsVectorCompletionsResponseClearResponseSchemaExecute = logsVectorCompletionsResponseClearResponseSchemaExecute;
exports.logsVectorCompletionsResponseClearResponseSchemaExecuteJq = logsVectorCompletionsResponseClearResponseSchemaExecuteJq;
exports.logsVectorCompletionsResponseGetExecute = logsVectorCompletionsResponseGetExecute;
exports.logsVectorCompletionsResponseGetExecuteJq = logsVectorCompletionsResponseGetExecuteJq;
exports.logsVectorCompletionsResponseGetRequestSchemaExecute = logsVectorCompletionsResponseGetRequestSchemaExecute;
exports.logsVectorCompletionsResponseGetRequestSchemaExecuteJq = logsVectorCompletionsResponseGetRequestSchemaExecuteJq;
exports.logsVectorCompletionsResponseGetResponseSchemaExecute = logsVectorCompletionsResponseGetResponseSchemaExecute;
exports.logsVectorCompletionsResponseGetResponseSchemaExecuteJq = logsVectorCompletionsResponseGetResponseSchemaExecuteJq;
exports.logsVectorCompletionsResponseListExecute = logsVectorCompletionsResponseListExecute;
exports.logsVectorCompletionsResponseListExecuteJq = logsVectorCompletionsResponseListExecuteJq;
exports.logsVectorCompletionsResponseListRequestSchemaExecute = logsVectorCompletionsResponseListRequestSchemaExecute;
exports.logsVectorCompletionsResponseListRequestSchemaExecuteJq = logsVectorCompletionsResponseListRequestSchemaExecuteJq;
exports.logsVectorCompletionsResponseListResponseSchemaExecute = logsVectorCompletionsResponseListResponseSchemaExecute;
exports.logsVectorCompletionsResponseListResponseSchemaExecuteJq = logsVectorCompletionsResponseListResponseSchemaExecuteJq;
exports.logsVectorCompletionsResponseSubscribeExecute = logsVectorCompletionsResponseSubscribeExecute;
exports.logsVectorCompletionsResponseSubscribeExecuteJq = logsVectorCompletionsResponseSubscribeExecuteJq;
exports.logsVectorCompletionsResponseSubscribeRequestSchemaExecute = logsVectorCompletionsResponseSubscribeRequestSchemaExecute;
exports.logsVectorCompletionsResponseSubscribeRequestSchemaExecuteJq = logsVectorCompletionsResponseSubscribeRequestSchemaExecuteJq;
exports.logsVectorCompletionsResponseSubscribeResponseSchemaExecute = logsVectorCompletionsResponseSubscribeResponseSchemaExecute;
exports.logsVectorCompletionsResponseSubscribeResponseSchemaExecuteJq = logsVectorCompletionsResponseSubscribeResponseSchemaExecuteJq;
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
