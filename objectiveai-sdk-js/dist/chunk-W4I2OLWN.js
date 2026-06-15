import { z } from 'zod';

// src/viewer/event.ts
var JsonValueSchema = z.union([
  z.string(),
  z.number(),
  z.boolean(),
  z.null(),
  z.array(z.lazy(() => JsonValueSchema)),
  z.record(z.string(), z.lazy(() => JsonValueSchema))
]);

// src/viewer/event.ts
var ViewerEventSchema = z.union([z.object({
  destination: z.string(),
  sub_type: z.string(),
  type: z.literal("inbound"),
  value: JsonValueSchema
}).describe("Host \u2192 iframe. Carries data into the plugin (the existing\npath). `sub_type` is the snake_case discriminator the plugin\nlistens on (built-ins: `agent_completions` /\n`functions_executions`; plugins: whatever they declared in\ntheir manifest's `viewer_routes[i].type`).").meta({ "variantTitle": "Inbound" }), z.object({
  destination: z.string(),
  type: z.literal("cli_command"),
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
var CliCommandAgentsEnqueueResponseSchema = z.object({
  agent_instance_hierarchy: z.string().nullable().meta({ omitempty: true }).optional(),
  agent_tag: z.string().nullable().meta({ omitempty: true }).optional(),
  id: z.number().int().min(-9223372036854776e3).max(9223372036854776e3)
}).describe("The freshly parked queue row: its id and the target it's scoped\nto (exactly one of `agent_instance_hierarchy` / `agent_tag` is\nset).").meta({ title: "cli.command.agents.enqueue.Response" });
var CliErrorTypeSchema = z.literal("error").describe('Single-variant discriminator for [`Error`]\'s `type` field.\nAlways `"error"` on the wire.').meta({ title: "cli.ErrorType" });
var CliLevelSchema = z.enum(["trace", "debug", "info", "warn", "error"]).describe("Severity matching the conventions used by bunyan / pino / `log` crate\nJSON encoders. `fatal` is encoded separately on [`Error`].").meta({ title: "cli.Level" });

// src/cli/error.ts
var CliErrorSchema = z.object({
  fatal: z.boolean().nullable().meta({ omitempty: true }).optional(),
  level: CliLevelSchema.nullable().meta({ omitempty: true }).optional(),
  message: JsonValueSchema,
  type: CliErrorTypeSchema
}).describe('A failure or advisory written to stdout. `fatal: true` means the\nprocess is exiting with a non-zero status; `fatal: false` is a\nnon-blocking warning (e.g. auto-update failed but the requested\ncommand still ran).\n\n`message` is an arbitrary JSON value so producers can emit\nstructured payloads (e.g. `{"code": ..., "detail": ...}`). Wrap\na plain string as `Value::String(...)` (or use `.into()`) and the\nwire bytes stay identical to the old `String`-only shape.\n\nThe `type` field is a single-variant `ErrorType` enum that\nalways serializes to `"error"`. This is what disambiguates the\nuntagged `Output` enum from a notification \u2014 `Output` tries\n`Error` first, and the constant `type:"error"` tag is what\nrejects every non-error wire shape.').meta({ title: "cli.Error" });

// src/viewer/command/agents/enqueue.ts
async function agentsEnqueueExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/enqueue" }), z.union([CliErrorSchema, CliCommandAgentsEnqueueResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents enqueue: cli produced no output before the end marker");
  }
  return first;
}
async function agentsEnqueueExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/enqueue" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents enqueue: cli produced no output before the end marker");
  }
  return first;
}
async function agentsEnqueueRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/enqueue/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents enqueue request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsEnqueueRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/enqueue/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents enqueue request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsEnqueueResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/enqueue/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents enqueue response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsEnqueueResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/enqueue/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents enqueue response_schema: cli produced no output before the end marker");
  }
  return first;
}
var RemotePathSchema = z.union([z.object({
  commit: z.string(),
  owner: z.string(),
  remote: z.literal("github"),
  repository: z.string()
}).meta({ "variantTitle": "Github" }), z.object({
  commit: z.string(),
  owner: z.string(),
  remote: z.literal("client"),
  repository: z.string()
}).meta({ "variantTitle": "Client" }), z.object({
  name: z.string(),
  remote: z.literal("mock")
}).meta({ "variantTitle": "Mock" })]).meta({ title: "RemotePath" });

// src/cli/command/agents/get/response.ts
var CliCommandAgentsGetResponseSchema = RemotePathSchema.and(z.object({})).describe("Response for `agents get` \u2014 an Agent base definition with its remote path.").meta({ title: "cli.command.agents.get.Response" });

// src/viewer/command/agents/get.ts
async function agentsGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/get" }), z.union([CliErrorSchema, CliCommandAgentsGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents get: cli produced no output before the end marker");
  }
  return first;
}
async function agentsGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents get: cli produced no output before the end marker");
  }
  return first;
}
async function agentsGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandAgentsInstancesListResponseItemSchema = z.object({
  agent_instance_hierarchy: z.string().describe("Full hierarchy of this agent instance."),
  created_at: z.string().nullable().describe("RFC3339 timestamp of the first `logs.messages` row for this\nagent. `None` when the agent has no logs yet (queue-only).").meta({ omitempty: true }).optional(),
  last_active_at: z.string().nullable().describe("RFC3339 timestamp of the most recent `logs.messages` row for\nthis agent. `None` when the agent has no logs yet (queue-only).").meta({ omitempty: true }).optional(),
  logged: z.number().int().min(0).max(18446744073709552e3).describe("Total `logs.messages` rows for this agent over all time."),
  queued: z.number().int().min(0).max(18446744073709552e3).describe("Active `message_queue` rows targeting this agent \u2014 counting\nboth direct-AIH rows and rows whose tag is bound to this AIH."),
  tags: z.array(z.string()).describe("Tag names currently bound to this AIH, newest-bound first.")
}).describe("One discovered agent instance under a target. Aggregated from the\n`logs.messages`, `message_queue`, and `tags` tiers.").meta({ title: "cli.command.agents.instances.list.ResponseItem" });

// src/viewer/command/agents/instances/get.ts
function agentsInstancesGetExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/instances/get" }), z.union([CliErrorSchema, CliCommandAgentsInstancesListResponseItemSchema]));
}
function agentsInstancesGetExecuteTransform(request, transform) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/instances/get" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function agentsInstancesGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/instances/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents instances get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsInstancesGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/instances/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents instances get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsInstancesGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/instances/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents instances get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsInstancesGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/instances/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents instances get response_schema: cli produced no output before the end marker");
  }
  return first;
}
function agentsInstancesListExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/instances/list" }), z.union([CliErrorSchema, CliCommandAgentsInstancesListResponseItemSchema]));
}
function agentsInstancesListExecuteTransform(request, transform) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/instances/list" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function agentsInstancesListRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/instances/list/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents instances list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsInstancesListRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/instances/list/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents instances list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsInstancesListResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/instances/list/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents instances list response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsInstancesListResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/instances/list/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents instances list response_schema: cli produced no output before the end marker");
  }
  return first;
}
function agentsListExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/list" }), z.union([CliErrorSchema, RemotePathSchema]));
}
function agentsListExecuteTransform(request, transform) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/list" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function agentsListRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/list/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsListRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/list/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsListResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/list/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents list response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsListResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/list/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents list response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandAgentsLogsReadAllAssistantResponsePartSchema = z.union([z.object({
  delivered_at: z.string(),
  function_name: z.string().describe("`objectiveai.assistant_response_tool_calls.function_name`."),
  id: z.number().int().min(-9223372036854776e3).max(9223372036854776e3).describe('`logs.messages."index"` for the tool-call row. Pass to\n`agents logs read id <n>` to read the call\'s `arguments`\nas text.'),
  tool_call_id: z.string().describe("The wire tool-call id this row carries."),
  tool_call_index: z.number().int().min(-9223372036854776e3).max(9223372036854776e3).describe("The tool call's wire index within the assistant message's\n`tool_calls[]`."),
  type: z.literal("tool_call")
}).meta({ "variantTitle": "ToolCall" }), z.object({
  delivered_at: z.string(),
  id: z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  type: z.literal("refusal")
}).meta({ "variantTitle": "Refusal" }), z.object({
  delivered_at: z.string(),
  id: z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  type: z.literal("reasoning")
}).meta({ "variantTitle": "Reasoning" }), z.object({
  delivered_at: z.string(),
  id: z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  type: z.literal("text")
}).meta({ "variantTitle": "Text" }), z.object({
  delivered_at: z.string(),
  id: z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  type: z.literal("image")
}).meta({ "variantTitle": "Image" }), z.object({
  delivered_at: z.string(),
  id: z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  type: z.literal("audio")
}).meta({ "variantTitle": "Audio" }), z.object({
  delivered_at: z.string(),
  id: z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  type: z.literal("video")
}).meta({ "variantTitle": "Video" }), z.object({
  delivered_at: z.string(),
  id: z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  type: z.literal("file")
}).meta({ "variantTitle": "File" })]).describe("One row inside an `AssistantResponse` block, tagged by the\ntable-kind of the underlying `assistant_response_*` row.\n\nThe `ToolCall` variant inlines the call's metadata\n(`function_name` / `tool_call_id` / `tool_call_index`) so callers\ncan dedupe and correlate without a per-row round-trip; its `id`\naddresses the same `assistant_response_tool_calls` row, which\n`agents logs read id <id>` returns as the call's `arguments`\n(text). Every other variant carries only `id` (the\n`logs.messages.\"index\"` to pass to `agents logs read id`) and the\ndelivery timestamp.").meta({ title: "cli.command.agents.logs.read.all.AssistantResponsePart" });
var CliCommandAgentsLogsReadAllClientNotificationPartTypeSchema = z.enum(["text", "image", "audio", "video", "file"]).describe("Type tag for one `ClientNotification` part \u2014 the table-kind of\nthe underlying `message_queue_*` content row.").meta({ title: "cli.command.agents.logs.read.all.ClientNotificationPartType" });

// src/cli/command/agents/logs/read/all/clientNotificationPart.ts
var CliCommandAgentsLogsReadAllClientNotificationPartSchema = z.object({
  delivered_at: z.string().describe('`logs.messages."timestamp"` \u2014 when the receiver consumed\nthis content row and the LogWriter committed the\nconsumption event.'),
  id: z.number().int().min(-9223372036854776e3).max(9223372036854776e3).describe('`logs.messages."index"` for this row. Pass to\n`agents logs read id <n>` to fetch the consumed\n`message_queue_contents` body.'),
  type: CliCommandAgentsLogsReadAllClientNotificationPartTypeSchema
}).describe("One row inside a `ClientNotification` block \u2014 a consumed\n`message_queue_contents` entry. `queued_at` is on the\nenclosing block (it lives on `message_queue.enqueued_at`, not\nper-content); only the per-row consumption timestamp is here.").meta({ title: "cli.command.agents.logs.read.all.ClientNotificationPart" });
var CliCommandAgentsLogsReadAllToolResponsePartTypeSchema = z.enum(["text", "image", "audio", "video", "file"]).describe("Type tag for one `ToolResponse` part \u2014 the table-kind of the\nunderlying `tool_response_content_*` row. The tool-call linkage\n(`tool_call_id`) lives on the enclosing `ResponseItem::ToolResponse`\nblock, not on the parts.").meta({ title: "cli.command.agents.logs.read.all.ToolResponsePartType" });

// src/cli/command/agents/logs/read/all/toolResponsePart.ts
var CliCommandAgentsLogsReadAllToolResponsePartSchema = z.object({
  delivered_at: z.string(),
  id: z.number().int().min(-9223372036854776e3).max(9223372036854776e3).describe('`logs.messages."index"` for this row. Pass to\n`agents logs read id <n>` for the typed body.'),
  type: CliCommandAgentsLogsReadAllToolResponsePartTypeSchema
}).describe("One row inside a `ToolResponse` block.").meta({ title: "cli.command.agents.logs.read.all.ToolResponsePart" });

// src/cli/command/agents/logs/read/all/responseItem.ts
var CliCommandAgentsLogsReadAllResponseItemSchema = z.union([z.object({
  agent_instance_hierarchy: z.string(),
  delivered_at: z.string(),
  id: z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  response_id: z.string(),
  sender_agent_instance_hierarchy: z.string().describe("AIH of the caller who issued the request \u2014 from\n`logs.agent_completion_requests.sender_*`."),
  type: z.literal("agent_completion_request")
}).meta({ "variantTitle": "AgentCompletionRequest" }), z.object({
  agent_instance_hierarchy: z.string(),
  delivered_at: z.string(),
  id: z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  response_id: z.string(),
  sender_agent_instance_hierarchy: z.string(),
  type: z.literal("vector_completion_request")
}).meta({ "variantTitle": "VectorCompletionRequest" }), z.object({
  agent_instance_hierarchy: z.string(),
  delivered_at: z.string(),
  id: z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  response_id: z.string(),
  sender_agent_instance_hierarchy: z.string(),
  type: z.literal("function_execution_request")
}).meta({ "variantTitle": "FunctionExecutionRequest" }), z.object({
  agent_instance_hierarchy: z.string(),
  key: z.string().nullable().describe("Idempotency token, if the row was enqueued with\n`--key` via `agents message --enqueue-with-key`.\nSurfacing it lets readers attribute a notification\nto a specific enqueue beyond just the sender AIH.").meta({ omitempty: true }).optional(),
  parts: z.array(CliCommandAgentsLogsReadAllClientNotificationPartSchema),
  queued_at: z.string().describe("`message_queue.enqueued_at` of the consumed parent\nqueue row. One block = one parent queue row, so this\nis well-defined block-level (each part's individual\n`delivered_at` still records its own\nconsumption moment)."),
  response_id: z.string(),
  sender_agent_instance_hierarchy: z.string().describe("AIH of the enqueuer \u2014 from `message_queue.sender_*`\njoined through `message_queue_contents.id`."),
  type: z.literal("client_notification")
}).meta({ "variantTitle": "ClientNotification" }), z.object({
  agent_instance_hierarchy: z.string(),
  parts: z.array(CliCommandAgentsLogsReadAllAssistantResponsePartSchema),
  response_id: z.string(),
  type: z.literal("assistant_response")
}).describe("Agent emissions \u2014 the agent IS the producer of these\nrows, so there's no separate sender. The\n`agent_instance_hierarchy` field IS the sender.").meta({ "variantTitle": "AssistantResponse" }), z.object({
  agent_instance_hierarchy: z.string(),
  parts: z.array(CliCommandAgentsLogsReadAllToolResponsePartSchema),
  response_id: z.string(),
  tool_call_id: z.string().describe("The wire tool-call id this response answers."),
  type: z.literal("tool_response")
}).describe("One tool call's response. Blocks are grouped per\n`tool_call_id` (in addition to `agent_instance_hierarchy` +\n`response_id`), so two responses in the same turn yield two\nblocks.").meta({ "variantTitle": "ToolResponse" })]).describe("One yielded item. Three single-row request blobs +\nthree multi-row blocks. Every variant carries `response_id`.\n`sender_agent_instance_hierarchy` appears only on the four\nvariants that have a sender \u2260 producer: the three request\nvariants (caller AIH) and `ClientNotification` (enqueuer\nAIH). `AssistantResponse` and `ToolResponse` are emitted BY\nthe agent itself \u2014 their `agent_instance_hierarchy` IS the\nproducer, so no separate sender field exists.\n\nBlock-coalescing boundary tuple: `(class,\nagent_instance_hierarchy, response_id)` for `AssistantResponse`;\n`(class, agent_instance_hierarchy, response_id, tool_call_id)`\nfor `ToolResponse` (one block per tool call); `(class,\nagent_instance_hierarchy, response_id, sender, message_queue_id)`\nfor `ClientNotification` blocks.\nOne `ClientNotification` block = one consumed\n`message_queue` parent row, so `queued_at` and\n`sender_agent_instance_hierarchy` are well-defined\nblock-level.").meta({ title: "cli.command.agents.logs.read.all.ResponseItem" });

// src/viewer/command/agents/logs/read/all.ts
function agentsLogsReadAllExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/logs/read/all" }), z.union([CliErrorSchema, CliCommandAgentsLogsReadAllResponseItemSchema]));
}
function agentsLogsReadAllExecuteTransform(request, transform) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/logs/read/all" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function agentsLogsReadAllRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/logs/read/all/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read all request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsLogsReadAllRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/logs/read/all/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read all request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsLogsReadAllResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/logs/read/all/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read all response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsLogsReadAllResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/logs/read/all/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read all response_schema: cli produced no output before the end marker");
  }
  return first;
}
var AgentCompletionsMessageFileSchema = z.object({
  file_data: z.string().nullable().describe("Base64-encoded file data.").meta({ omitempty: true }).optional(),
  file_id: z.string().nullable().describe("The ID of a previously uploaded file.").meta({ omitempty: true }).optional(),
  file_url: z.string().nullable().describe("A URL to fetch the file from.").meta({ omitempty: true }).optional(),
  filename: z.string().nullable().describe("The filename for display purposes.").meta({ omitempty: true }).optional()
}).describe("A file attachment for multimodal input.").meta({ title: "agent.completions.message.File" });
var AgentCompletionsMessageImageUrlDetailSchema = z.union([z.literal("auto").describe("Let the model decide the detail level.").meta({ "variantTitle": "Auto" }), z.literal("low").describe("Low detail mode (faster, less tokens).").meta({ "variantTitle": "Low" }), z.literal("high").describe("High detail mode (more accurate, more tokens).").meta({ "variantTitle": "High" })]).describe("Detail level for image processing.").meta({ title: "agent.completions.message.ImageUrlDetail" });

// src/agent/completions/message/imageUrl.ts
var AgentCompletionsMessageImageUrlSchema = z.object({
  detail: AgentCompletionsMessageImageUrlDetailSchema.nullable().describe("The detail level for image processing.").meta({ omitempty: true }).optional(),
  url: z.string().describe("The URL of the image (can be a data URL or HTTP URL).")
}).describe("An image URL for multimodal input.").meta({ title: "agent.completions.message.ImageUrl" });
var AgentCompletionsMessageInputAudioSchema = z.object({
  data: z.string().describe("Base64-encoded audio data."),
  format: z.string().describe('The audio format (e.g., "wav", "mp3").')
}).describe("Audio input for multimodal messages.").meta({ title: "agent.completions.message.InputAudio" });
var AgentCompletionsMessageVideoUrlSchema = z.object({
  url: z.string().describe("The URL of the video.")
}).describe("A video URL for multimodal input.").meta({ title: "agent.completions.message.VideoUrl" });
var AgentCompletionsMessageAssistantToolCallFunctionSchema = z.object({
  arguments: z.string().describe("The arguments to pass to the function, as a JSON string."),
  name: z.string().describe("The name of the function to call.")
}).describe("Details of a function call made by the assistant.").meta({ title: "agent.completions.message.AssistantToolCallFunction" });

// src/agent/completions/message/assistantToolCall.ts
var AgentCompletionsMessageAssistantToolCallSchema = z.object({
  function: AgentCompletionsMessageAssistantToolCallFunctionSchema.describe("The function being called."),
  id: z.string().describe("The unique ID of this tool call."),
  type: z.literal("function")
}).describe("A function call with an ID and function details.").meta({ title: "agent.completions.message.AssistantToolCall" });
var AgentCompletionsMessageRichContentPartSchema = z.union([z.object({
  text: z.string(),
  type: z.literal("text")
}).describe("Text content.").meta({ "variantTitle": "Text" }), z.object({
  image_url: AgentCompletionsMessageImageUrlSchema,
  type: z.literal("image_url")
}).describe("An image URL.").meta({ "variantTitle": "ImageUrl" }), z.object({
  input_audio: AgentCompletionsMessageInputAudioSchema,
  type: z.literal("input_audio")
}).describe("Audio input.").meta({ "variantTitle": "InputAudio" }), z.object({
  type: z.literal("input_video"),
  video_url: AgentCompletionsMessageVideoUrlSchema
}).describe("Video input.").meta({ "variantTitle": "InputVideo" }), z.object({
  type: z.literal("video_url"),
  video_url: AgentCompletionsMessageVideoUrlSchema
}).describe("A video URL.").meta({ "variantTitle": "VideoUrl" }), z.object({
  file: AgentCompletionsMessageFileSchema,
  type: z.literal("file")
}).describe("A file.").meta({ "variantTitle": "File" })]).describe("A part of rich content.").meta({ title: "agent.completions.message.RichContentPart" });

// src/agent/completions/message/richContent.ts
var AgentCompletionsMessageRichContentSchema = z.union([z.string().describe("Plain text content.").meta({ "variantTitle": "Text" }), z.array(AgentCompletionsMessageRichContentPartSchema).describe("Multi-part content (text, images, audio, video, files).").meta({ "variantTitle": "Parts" })]).describe("Rich content for user/assistant messages (supports multimodal input).").meta({ title: "agent.completions.message.RichContent" });

// src/agent/completions/message/assistantMessage.ts
var AgentCompletionsMessageAssistantMessageSchema = z.object({
  content: AgentCompletionsMessageRichContentSchema.nullable().describe("The message content, if any.").meta({ omitempty: true }).optional(),
  name: z.string().nullable().describe("Optional name for the assistant.").meta({ omitempty: true }).optional(),
  reasoning: z.string().nullable().describe("Reasoning content from models that support chain-of-thought.").meta({ omitempty: true }).optional(),
  refusal: z.string().nullable().describe("Refusal message if the model declined to respond.").meta({ omitempty: true }).optional(),
  tool_calls: z.array(AgentCompletionsMessageAssistantToolCallSchema).nullable().describe("Tool calls made by the assistant.").meta({ omitempty: true }).optional()
}).describe("An assistant message (model's previous response).").meta({ title: "agent.completions.message.AssistantMessage" });
var AgentCompletionsMessageSimpleContentPartSchema = z.object({
  text: z.string().describe("The text content."),
  type: z.literal("text")
}).describe("A text part.").meta({ title: "agent.completions.message.SimpleContentPart" });

// src/agent/completions/message/simpleContent.ts
var AgentCompletionsMessageSimpleContentSchema = z.union([z.string().describe("Plain text content.").meta({ "variantTitle": "Text" }), z.array(AgentCompletionsMessageSimpleContentPartSchema).describe("Multi-part text content.").meta({ "variantTitle": "Parts" })]).describe("Simple text content for system/developer messages.").meta({ title: "agent.completions.message.SimpleContent" });

// src/agent/completions/message/developerMessage.ts
var AgentCompletionsMessageDeveloperMessageSchema = z.object({
  content: AgentCompletionsMessageSimpleContentSchema.describe("The message content."),
  name: z.string().nullable().describe("Optional name for the message author.").meta({ omitempty: true }).optional()
}).describe("A developer message.").meta({ title: "agent.completions.message.DeveloperMessage" });
var AgentCompletionsMessageSystemMessageSchema = z.object({
  content: AgentCompletionsMessageSimpleContentSchema.describe("The message content."),
  name: z.string().nullable().describe("Optional name for the message author.").meta({ omitempty: true }).optional()
}).describe("A system message setting context or instructions.").meta({ title: "agent.completions.message.SystemMessage" });
var AgentCompletionsMessageToolResponseMetadataSchema = z.object({
  notifications: z.number().int().min(0).max(18446744073709552e3).nullable().describe("Count of pending notifications the proxy drained and prepended\nto the tool response's `content` before returning. Only set\nwhen at least one notification was drained.").meta({ omitempty: true }).optional()
}).describe("Vendor-extension metadata attached to a tool response. The\n`objectiveai-mcp-proxy` populates known keys (currently\n`notifications`); the SDK lossy-decodes the MCP `_meta` bag into\nthis typed shape. Unknown keys are dropped.").meta({ title: "agent.completions.message.ToolResponseMetadata" });

// src/agent/completions/message/toolMessage.ts
var AgentCompletionsMessageToolMessageSchema = z.object({
  content: AgentCompletionsMessageRichContentSchema.describe("The content of the tool response."),
  metadata: AgentCompletionsMessageToolResponseMetadataSchema.nullable().describe("Optional vendor-extension metadata, populated by\n`objectiveai-mcp-proxy` via MCP's `_meta` extension bag.").meta({ omitempty: true }).optional(),
  tool_call_id: z.string().describe("The ID of the tool call this message responds to.")
}).describe("A tool message containing the result of a tool call.").meta({ title: "agent.completions.message.ToolMessage" });
var AgentCompletionsMessageUserMessageSchema = z.object({
  content: AgentCompletionsMessageRichContentSchema.describe("The message content (supports text, images, audio, video, files)."),
  name: z.string().nullable().describe("Optional name for the user.").meta({ omitempty: true }).optional()
}).describe("A user message from the end user.").meta({ title: "agent.completions.message.UserMessage" });

// src/agent/completions/message/message.ts
var AgentCompletionsMessageMessageSchema = z.union([AgentCompletionsMessageDeveloperMessageSchema.and(z.object({
  role: z.literal("developer")
})).describe("A developer message (similar to system, but from the developer).").meta({ "variantTitle": "Developer" }), AgentCompletionsMessageSystemMessageSchema.and(z.object({
  role: z.literal("system")
})).describe("A system message setting context or instructions.").meta({ "variantTitle": "System" }), AgentCompletionsMessageUserMessageSchema.and(z.object({
  role: z.literal("user")
})).describe("A user message from the end user.").meta({ "variantTitle": "User" }), AgentCompletionsMessageAssistantMessageSchema.and(z.object({
  role: z.literal("assistant")
})).describe("An assistant message (model's previous response).").meta({ "variantTitle": "Assistant" }), AgentCompletionsMessageToolMessageSchema.and(z.object({
  role: z.literal("tool")
})).describe("A tool message containing the result of a tool call.").meta({ "variantTitle": "Tool" })]).describe("A message in the conversation.").meta({ title: "agent.completions.message.Message" });
var AgentCompletionsRequestProviderDataCollectionSchema = z.union([z.literal("deny").describe("Do not allow data collection.").meta({ "variantTitle": "Deny" }), z.literal("allow").describe("Allow data collection.").meta({ "variantTitle": "Allow" })]).describe("Data collection policy for providers.").meta({ title: "agent.completions.request.ProviderDataCollection" });
var AgentCompletionsRequestProviderMaxPriceSchema = z.object({
  audio: z.number().min(-34028234663852886e22).max(34028234663852886e22).nullable().describe("Maximum price per audio second.").meta({ omitempty: true }).optional(),
  completion: z.number().min(-34028234663852886e22).max(34028234663852886e22).nullable().describe("Maximum price per completion token.").meta({ omitempty: true }).optional(),
  image: z.number().min(-34028234663852886e22).max(34028234663852886e22).nullable().describe("Maximum price per image.").meta({ omitempty: true }).optional(),
  prompt: z.number().min(-34028234663852886e22).max(34028234663852886e22).nullable().describe("Maximum price per prompt token.").meta({ omitempty: true }).optional(),
  request: z.number().min(-34028234663852886e22).max(34028234663852886e22).nullable().describe("Maximum price per request.").meta({ omitempty: true }).optional()
}).describe("Maximum price constraints per token type.").meta({ title: "agent.completions.request.ProviderMaxPrice" });
var AgentCompletionsRequestProviderSortSchema = z.union([z.literal("price").describe("Prioritize by price (cheapest first).").meta({ "variantTitle": "Price" }), z.literal("throughput").describe("Prioritize by throughput (fastest first).").meta({ "variantTitle": "Throughput" }), z.literal("latency").describe("Prioritize by latency (lowest first).").meta({ "variantTitle": "Latency" })]).describe("How to sort/prioritize providers.").meta({ title: "agent.completions.request.ProviderSort" });

// src/agent/completions/request/provider.ts
var AgentCompletionsRequestProviderSchema = z.object({
  data_collection: AgentCompletionsRequestProviderDataCollectionSchema.nullable().describe("Whether to allow providers to collect data.").meta({ omitempty: true }).optional(),
  max_latency: z.number().min(-34028234663852886e22).max(34028234663852886e22).nullable().describe("Hard maximum latency requirement (seconds).").meta({ omitempty: true }).optional(),
  max_price: AgentCompletionsRequestProviderMaxPriceSchema.nullable().describe("Maximum price constraints.").meta({ omitempty: true }).optional(),
  min_throughput: z.number().min(-34028234663852886e22).max(34028234663852886e22).nullable().describe("Hard minimum throughput requirement (tokens/second).").meta({ omitempty: true }).optional(),
  preferred_max_latency: z.number().min(-34028234663852886e22).max(34028234663852886e22).nullable().describe("Preferred maximum latency (seconds).").meta({ omitempty: true }).optional(),
  preferred_min_throughput: z.number().min(-34028234663852886e22).max(34028234663852886e22).nullable().describe("Preferred minimum throughput (tokens/second).").meta({ omitempty: true }).optional(),
  sort: AgentCompletionsRequestProviderSortSchema.nullable().describe("How to sort/prioritize providers.").meta({ omitempty: true }).optional(),
  zdr: z.boolean().nullable().describe("Whether to use zero data retention providers only.").meta({ omitempty: true }).optional()
}).describe("Provider routing and selection preferences.").meta({ title: "agent.completions.request.Provider" });
var AgentCompletionsRequestResponseFormatSchema = z.union([z.object({
  type: z.literal("text")
}).describe("Plain text response (default).").meta({ "variantTitle": "Text" }), z.object({
  type: z.literal("json_object")
}).describe("Response must be valid JSON.").meta({ "variantTitle": "JsonObject" }), z.object({
  schema: z.record(z.string(), JsonValueSchema).describe("The JSON Schema definition."),
  type: z.literal("json_schema")
}).describe("Response must conform to a JSON schema.").meta({ "variantTitle": "JsonSchema" }), z.object({
  grammar: z.string(),
  type: z.literal("grammar")
}).describe("Response must conform to a grammar.").meta({ "variantTitle": "Grammar" }), z.object({
  type: z.literal("python")
}).describe("Response must be valid Python code.").meta({ "variantTitle": "Python" }), z.object({
  description: z.string().describe("A description of the tool."),
  name: z.string().describe("The name of the tool."),
  required: z.boolean().nullable().describe("Whether the tool MUST be called.").meta({ omitempty: true }).optional(),
  schema: z.record(z.string(), JsonValueSchema).describe("The JSON Schema definition."),
  type: z.literal("tool_call")
}).describe("The final assistant message will contain this tool call").meta({ "variantTitle": "ToolCall" })]).describe("The format of the model's response.").meta({ title: "agent.completions.request.ResponseFormat" });

// src/agent/completions/request/responseFormatParam.ts
var AgentCompletionsRequestResponseFormatParamSchema = z.union([AgentCompletionsRequestResponseFormatSchema.describe("A single response format applied to all agents.").meta({ "title": "agent.completions.request.ResponseFormat", "variantTitle": "Single" }), z.record(z.string(), AgentCompletionsRequestResponseFormatSchema).describe("Per-agent response formats, keyed by agent ID.").meta({ "variantTitle": "PerAgent" })]).describe("Either a single response format or a per-agent map.").meta({ title: "agent.completions.request.ResponseFormatParam" });
var AgentClaudeAgentSdkEffortSchema = z.union([z.literal("low").describe("Minimal output, concise responses.").meta({ "variantTitle": "Low" }), z.literal("medium").describe("Balanced output (default, normalized away during preparation).").meta({ "variantTitle": "Medium" }), z.literal("high").describe("Detailed output with thorough explanations.").meta({ "variantTitle": "High" }), z.literal("xhigh").describe("Extra-high effort, above `High` but below `Max`.").meta({ "variantTitle": "Xhigh" }), z.literal("max").describe("Maximum effort, most detailed output possible.").meta({ "variantTitle": "Max" })]).describe("The effort level for model output.\n\nThis setting hints to the model how detailed its responses should be.").meta({ title: "agent.claude_agent_sdk.Effort" });
var AgentClaudeAgentSdkOutputModeSchema = z.literal("instruction").describe("The model is instructed via the prompt to output a specific key.\n\nThis is the default and most widely supported mode.").meta({ title: "agent.claude_agent_sdk.OutputMode" });
var AgentClaudeAgentSdkUpstreamSchema = z.literal("claude_agent_sdk").describe("Claude Agent SDK upstream marker.").meta({ title: "agent.claude_agent_sdk.Upstream" });
var AgentClientObjectiveaiMcpEntrySchema = z.object({
  name: z.string(),
  owner: z.string(),
  version: z.string()
}).describe("A single `owner` / `name` / `version` reference identifying one\ntool inside [`ClientObjectiveaiMcp::tools`]. Plugin references\nuse the larger [`ClientObjectiveaiMcpPluginEntry`] \u2014 they carry\nextra `executable` / `mcp_servers` fields tools don't have.").meta({ title: "agent.ClientObjectiveaiMcpEntry" });
var AgentClientObjectiveaiMcpPluginMcpServerSchema = z.object({
  arguments: z.record(z.string(), z.string().nullable()).nullable().describe('Optional key\u2192value arguments forwarded to the plugin alongside\n`name`. `Some(value)` \u21D2 `--key value` on the spawned plugin\'s\nargv; `None` \u21D2 a bare `--key` flag with no following token. The\nplugin author decides how to interpret them. [`prepare`]\nnormalizes (`Some("") \u2192 None`), sorts the map by key, and\ncollapses an empty map to `None` so two equivalent declarations\ncanonicalize to byte-identical JSON.').meta({ omitempty: true }).optional(),
  name: z.string().describe("Author-chosen identifier \u2014 must match a `name` in the plugin\nmanifest's `mcp_servers` list. The CLI feeds this as the\nfirst positional arg when starting the plugin.")
}).describe("One `mcp_servers` entry on a\n[`ClientObjectiveaiMcpPluginEntry`]: the manifest-declared `name`\nthe agent wants exposed, plus optional `arguments` the CLI feeds\nto the plugin alongside the name when bringing the server up. The\narguments map is sorted by key in [`prepare`] so two equivalent\ndeclarations (same key/value pairs in any order) hash to the same\ncanonical form.").meta({ title: "agent.ClientObjectiveaiMcpPluginMcpServer" });

// src/agent/clientObjectiveaiMcpPluginEntry.ts
var AgentClientObjectiveaiMcpPluginEntrySchema = z.object({
  executable: z.boolean().default(true).describe("`true`: spawn the plugin binary and surface its tools the\nusual way. `false`: don't run the plugin; only consume its\ndeclared MCP servers (`mcp_servers` below). Defaults to\n`true` so existing declarations keep their current behavior."),
  mcp_servers: z.array(AgentClientObjectiveaiMcpPluginMcpServerSchema).nullable().describe("Subset of the plugin's manifest `mcp_servers` to expose, each\nreferenced by `name` plus an optional `arguments` map. `None`\n\u21D2 none. Names that aren't present in the plugin's manifest are\nrejected when the API asks the CLI to begin them; declarations\nthemselves don't validate the referent (the plugin may not be\ninstalled at declaration time).").meta({ omitempty: true }).optional(),
  name: z.string(),
  owner: z.string(),
  version: z.string()
}).describe("Plugin reference inside [`ClientObjectiveaiMcp::plugins`].\n\n- `owner` / `name` / `version` identify the plugin (same shape as\n  [`ClientObjectiveaiMcpEntry`]).\n- `executable` controls whether this plugin contributes a tool\n  to the agent's surface (`true`, default) or is loaded purely\n  for its declared MCP servers (`false`). The API's\n  `X-OBJECTIVEAI-TOOLS-ALLOWED` construction honors this: only\n  `executable = true` plugin entries contribute their `name` to\n  the allow-list.\n- `mcp_servers` selects which of the plugin's manifest-declared\n  `filesystem::plugins::Manifest::mcp_servers` entries should be\n  exposed to the agent, by `name`. `None` \u21D2 none of them.").meta({ title: "agent.ClientObjectiveaiMcpPluginEntry" });

// src/agent/clientObjectiveaiMcp.ts
var AgentClientObjectiveaiMcpSchema = z.object({
  objectiveai: z.boolean().nullable().meta({ omitempty: true }).optional(),
  plugins: z.array(AgentClientObjectiveaiMcpPluginEntrySchema).meta({ omitempty: true }),
  tools: z.array(AgentClientObjectiveaiMcpEntrySchema).meta({ omitempty: true })
}).describe("Client-side MCP surface the agent expects:\n\n- `objectiveai`: whether the calling client exposes the built-in\n  `objectiveai-mcp`. `None` means unspecified; `Some(true)` /\n  `Some(false)` explicitly opt in / out.\n- `plugins`: specific plugins (by `owner` / `name` / `version`)\n  plus per-plugin `executable` + `mcp_servers`.\n- `tools`: specific tools (by `owner` / `name` / `version`).").meta({ title: "agent.ClientObjectiveaiMcp" });
var AgentMcpServerSchema = z.object({
  authorization: z.boolean().default(false).describe("Whether this MCP server uses authorization."),
  url: z.string().describe("The URL of the MCP server.")
}).describe("An MCP server that the agent can connect to.").meta({ title: "agent.McpServer" });

// src/agent/claude_agent_sdk/agentBase.ts
var AgentClaudeAgentSdkAgentBaseSchema = z.object({
  client_objectiveai_mcp: AgentClientObjectiveaiMcpSchema.nullable().describe("Client-side ObjectiveAI MCP surface the calling client is\nexpected to expose locally back to the API (objectiveai\nbuilt-in, plus specific plugins / tools by owner+name+version).").meta({ omitempty: true }).optional(),
  effort: AgentClaudeAgentSdkEffortSchema.nullable().describe("The effort level for model output.").meta({ omitempty: true }).optional(),
  mcp_servers: z.array(AgentMcpServerSchema).nullable().describe("MCP servers the agent can connect to.").meta({ omitempty: true }).optional(),
  model: z.string().describe("The upstream language model identifier."),
  output_mode: AgentClaudeAgentSdkOutputModeSchema.describe("The output mode for vector completions. Ignored for agent completions."),
  prefix_content: AgentCompletionsMessageRichContentSchema.nullable().describe("Rich content prepended to the user's prompt.").meta({ omitempty: true }).optional(),
  suffix_content: AgentCompletionsMessageRichContentSchema.nullable().describe("Rich content appended after the user's prompt.").meta({ omitempty: true }).optional(),
  system_prompt: z.string().nullable().describe("System prompt for the agent.").meta({ omitempty: true }).optional(),
  thinking: z.boolean().nullable().describe("Whether thinking/extended thinking is enabled.\n\nDefaults to `true`. Set to `false` to disable.").meta({ omitempty: true }).optional(),
  upstream: AgentClaudeAgentSdkUpstreamSchema.describe("The upstream provider marker.")
}).describe("The base configuration for a Claude Agent SDK Agent (without computed ID).").meta({ title: "agent.claude_agent_sdk.AgentBase" });
var AgentCodexSdkEffortSchema = z.union([z.literal("minimal").describe("Minimal reasoning, fastest responses.").meta({ "variantTitle": "Minimal" }), z.literal("low").describe("Low reasoning effort.").meta({ "variantTitle": "Low" }), z.literal("medium").describe("Balanced reasoning (default, normalized away during preparation).").meta({ "variantTitle": "Medium" }), z.literal("high").describe("High reasoning effort.").meta({ "variantTitle": "High" })]).describe("Reasoning-effort level for the Codex model. Maps to Codex's\n`model_reasoning_effort` (see `openai_codex_sdk` `ModelReasoningEffort`).\n\n`Medium` is the default and is normalized to `None` during preparation\nfor content-addressing stability.").meta({ title: "agent.codex_sdk.Effort" });
var AgentCodexSdkOutputModeSchema = z.literal("instruction").describe("The model is instructed via the prompt to output a specific key.\n\nThis is the default and most widely supported mode.").meta({ title: "agent.codex_sdk.OutputMode" });
var AgentCodexSdkUpstreamSchema = z.literal("codex_sdk").describe("Codex SDK upstream marker.").meta({ title: "agent.codex_sdk.Upstream" });

// src/agent/codex_sdk/agentBase.ts
var AgentCodexSdkAgentBaseSchema = z.object({
  client_objectiveai_mcp: AgentClientObjectiveaiMcpSchema.nullable().describe("Client-side ObjectiveAI MCP surface the calling client is\nexpected to expose locally back to the API (objectiveai\nbuilt-in, plus specific plugins / tools by owner+name+version).").meta({ omitempty: true }).optional(),
  effort: AgentCodexSdkEffortSchema.nullable().describe("Reasoning effort \u2014 maps to Codex's `model_reasoning_effort`.").meta({ omitempty: true }).optional(),
  mcp_servers: z.array(AgentMcpServerSchema).nullable().describe("MCP servers the agent can connect to.").meta({ omitempty: true }).optional(),
  model: z.string().describe("The upstream language model identifier (e.g. `gpt-5`)."),
  output_mode: AgentCodexSdkOutputModeSchema.describe("The output mode for vector completions. Ignored for agent completions."),
  prefix_content: AgentCompletionsMessageRichContentSchema.nullable().describe("Rich content prepended to the user's prompt.").meta({ omitempty: true }).optional(),
  suffix_content: AgentCompletionsMessageRichContentSchema.nullable().describe("Rich content appended after the user's prompt.").meta({ omitempty: true }).optional(),
  upstream: AgentCodexSdkUpstreamSchema.describe("The upstream provider marker."),
  web_search_enabled: z.boolean().nullable().describe("Whether this agent may use the codex binary's web-search tool.").meta({ omitempty: true }).optional()
}).describe("The base configuration for a Codex SDK Agent (without computed ID).").meta({ title: "agent.codex_sdk.AgentBase" });
var AgentMockCallToolCallSchema = z.object({
  arguments: z.string().describe("JSON-string arguments \u2014 same wire shape as\n`AssistantToolCallFunction::arguments`. Passed through to the\nemitted tool call verbatim; validation is the downstream\n`tools/call` handler's job."),
  name: z.string().describe("Tool name. Matched against the upstream tool surface at call\ntime the same way a model-emitted tool call would be.")
}).describe("A single tool call within a [`Call`]. Identifies the tool by\n`name` and carries the exact JSON-encoded arguments to pass. No\n`id` field \u2014 call ids on the wire are minted randomly per emit\nand are ignored by the override's match check.").meta({ title: "agent.mock.CallToolCall" });

// src/agent/mock/call.ts
var AgentMockCallSchema = z.object({
  content: z.string().describe('Assistant text content emitted *after* the tool calls. Plain\n`String` \u2014 the mock\'s chunk-emission path only produces\n`RichContent::Text`, so multimodal content was never possible\nhere anyway. An empty string is treated as "no content" by\nthe emitter (the wire shape becomes `content: None`) and\nmatches an assistant message with `content: None`.'),
  tool_calls: z.array(AgentMockCallToolCallSchema).describe('Tool calls emitted by this turn. Empty `Vec` is allowed and\nmeans "no tool calls, just content."').meta({ omitempty: true })
}).describe("One scripted assistant turn for the mock agent's deterministic\n`calls` override (see [`super::AgentBase::calls`]).\n\nWhen the mock fires this `Call`, it emits every entry in\n`tool_calls` first (as tool-call deltas), then `content` (as\ncontent deltas), finishing the turn after the last content chunk.\nThe match check on the continuation compares\n`(tool_calls[i].name, tool_calls[i].arguments)` pairs plus the\nfull `content` string \u2014 call ids are intentionally excluded since\nthey're random per-emit.").meta({ title: "agent.mock.Call" });
var AgentMockOutputModeSchema = z.union([z.literal("instruction").describe("The model is instructed via the prompt to output a specific key.\n\nThis is the default and most widely supported mode.").meta({ "variantTitle": "Instruction" }), z.literal("json_schema").describe("A JSON schema response format is used with an enum of possible keys.\n\nRequires model support for structured JSON output.").meta({ "variantTitle": "JsonSchema" }), z.literal("tool_call").describe("A forced tool call with an argument schema containing possible keys.\n\nRequires model support for tool/function calling.").meta({ "variantTitle": "ToolCall" })]).describe("The method used to constrain LLM output to valid response keys.\n\nIn vector completions, the model must select from a predefined set of\nresponses. This enum controls *how* that constraint is enforced.\n\n**Note:** This setting is only relevant for vector completions and is\ncompletely ignored for agent completions.").meta({ title: "agent.mock.OutputMode" });
var AgentMockUpstreamSchema = z.literal("mock").describe("Mock upstream marker.").meta({ title: "agent.mock.Upstream" });

// src/agent/mock/agentBase.ts
var AgentMockAgentBaseSchema = z.object({
  calls: z.array(AgentMockCallSchema).nullable().describe("Deterministic-script override. When `Some`, the mock agent\nemits each [`super::Call`] as its own assistant turn \u2014\n`tool_calls` first, then `content` \u2014 in array order. Each\nsubsequent turn inspects the continuation to count how many\n`Call`s have already been satisfied (assistant message with\nexactly that `Call`'s `tool_calls` (by name+arguments) and\n`content`); the next un-matched `Call` is what that turn\nemits. Once every `Call` has been satisfied in the\ncontinuation, the mock falls through to its normal\ndispatcher. Pure addition \u2014 agents without `calls` are\nunaffected.").meta({ omitempty: true }).optional(),
  client_objectiveai_mcp: AgentClientObjectiveaiMcpSchema.nullable().describe("Client-side ObjectiveAI MCP surface the calling client is\nexpected to expose locally back to the API (objectiveai\nbuilt-in, plus specific plugins / tools by owner+name+version).").meta({ omitempty: true }).optional(),
  error: z.boolean().nullable().describe("If true, the mock client will return an error instead of a response.").meta({ omitempty: true }).optional(),
  error_probability: z.number().int().min(0).max(255).nullable().describe("Probability (0-100) that the mock returns an error mid-stream.\nRequires `error` to be `Some(true)`.").meta({ omitempty: true }).optional(),
  mcp_servers: z.array(AgentMcpServerSchema).nullable().describe("MCP servers the agent can connect to.").meta({ omitempty: true }).optional(),
  output_mode: AgentMockOutputModeSchema.describe("The output mode for vector completions. Ignored for agent completions."),
  top_logprobs: z.number().int().min(0).max(18446744073709552e3).nullable().describe("Number of top log probabilities to return (2-20).\n\n**Vector completions only.** Ignored for agent completions.").meta({ omitempty: true }).optional(),
  upstream: AgentMockUpstreamSchema.describe("The upstream provider marker.")
}).describe("The base configuration for a Mock Agent (without computed ID).").meta({ title: "agent.mock.AgentBase" });
var AgentOpenrouterContextCompressionSchema = z.literal("middle-out").describe("Middle-out compression \u2014 drops content from the middle of the\nconversation when the request would otherwise exceed the\nmodel's context window. The only engine documented today.\n\nIntentionally no `#[schemars(title)]` here: the builder's\nsingle-variant-anyOf flatten merges the variant's title onto\nthe parent schema, clobbering the enum's `agent.openrouter\n.ContextCompression` rename. Add a per-variant title only\nonce a second variant lands.").meta({ title: "agent.openrouter.ContextCompression" });
var AgentOpenrouterOutputModeSchema = z.union([z.literal("instruction").describe("The model is instructed via the prompt to output a specific key.\n\nThis is the default and most widely supported mode.").meta({ "variantTitle": "Instruction" }), z.literal("json_schema").describe("A JSON schema response format is used with an enum of possible keys.\n\nRequires model support for structured JSON output.").meta({ "variantTitle": "JsonSchema" }), z.literal("tool_call").describe("A forced tool call with an argument schema containing possible keys.\n\nRequires model support for tool/function calling.").meta({ "variantTitle": "ToolCall" })]).describe("The method used to constrain LLM output to valid response keys.\n\nIn vector completions, the model must select from a predefined set of\nresponses. This enum controls *how* that constraint is enforced.\n\n**Note:** This setting is only relevant for vector completions and is\ncompletely ignored for agent completions.").meta({ title: "agent.openrouter.OutputMode" });
var AgentOpenrouterProviderQuantizationSchema = z.union([z.literal("int4").describe("4-bit integer quantization.").meta({ "variantTitle": "Int4" }), z.literal("int8").describe("8-bit integer quantization.").meta({ "variantTitle": "Int8" }), z.literal("fp4").describe("4-bit floating point quantization.").meta({ "variantTitle": "Fp4" }), z.literal("fp6").describe("6-bit floating point quantization.").meta({ "variantTitle": "Fp6" }), z.literal("fp8").describe("8-bit floating point quantization.").meta({ "variantTitle": "Fp8" }), z.literal("fp16").describe("16-bit floating point (half precision).").meta({ "variantTitle": "Fp16" }), z.literal("bf16").describe("16-bit brain floating point.").meta({ "variantTitle": "Bf16" }), z.literal("fp32").describe("32-bit floating point (full precision).").meta({ "variantTitle": "Fp32" }), z.literal("unknown").describe("Unknown quantization level.").meta({ "variantTitle": "Unknown" })]).describe("Model quantization levels for provider filtering.\n\nQuantization reduces model precision to decrease memory usage and\nincrease inference speed, potentially at the cost of output quality.").meta({ title: "agent.openrouter.ProviderQuantization" });

// src/agent/openrouter/provider.ts
var AgentOpenrouterProviderSchema = z.object({
  allow_fallbacks: z.boolean().nullable().describe("Whether to allow fallback to other providers if preferred ones fail.\nDefaults to `true`.").meta({ omitempty: true }).optional(),
  ignore: z.array(z.string()).nullable().describe("Providers to exclude from routing.").meta({ omitempty: true }).optional(),
  only: z.array(z.string()).nullable().describe("Exclusive list of allowed providers. If set, only these providers are used.").meta({ omitempty: true }).optional(),
  order: z.array(z.string()).nullable().describe("Preferred provider order. Earlier providers are tried first.").meta({ omitempty: true }).optional(),
  quantizations: z.array(AgentOpenrouterProviderQuantizationSchema).nullable().describe("Allowed model quantization levels.").meta({ omitempty: true }).optional(),
  require_parameters: z.boolean().nullable().describe("Whether to require that the provider supports all request parameters.\nDefaults to `false`.").meta({ omitempty: true }).optional()
}).describe("Provider routing preferences.\n\nControls which providers are used and in what order when routing\nrequests to upstream model hosts.").meta({ title: "agent.openrouter.Provider" });
var AgentOpenrouterReasoningEffortSchema = z.union([z.literal("none").describe("No reasoning.").meta({ "variantTitle": "None" }), z.literal("minimal").describe("Minimal reasoning effort.").meta({ "variantTitle": "Minimal" }), z.literal("low").describe("Low reasoning effort.").meta({ "variantTitle": "Low" }), z.literal("medium").describe("Medium reasoning effort.").meta({ "variantTitle": "Medium" }), z.literal("high").describe("High reasoning effort.").meta({ "variantTitle": "High" }), z.literal("xhigh").describe("Maximum reasoning effort.").meta({ "variantTitle": "Xhigh" })]).describe("The level of effort the model should put into reasoning.\n\nOnly supported by some models.").meta({ title: "agent.openrouter.ReasoningEffort" });
var AgentOpenrouterReasoningSummaryVerbositySchema = z.union([z.literal("auto").describe("Let the model decide (default, normalized away).").meta({ "variantTitle": "Auto" }), z.literal("concise").describe("Brief summary of reasoning.").meta({ "variantTitle": "Concise" }), z.literal("detailed").describe("Thorough summary of reasoning.").meta({ "variantTitle": "Detailed" })]).describe("Verbosity of the reasoning summary included in responses.\n\nOnly supported by some models.").meta({ title: "agent.openrouter.ReasoningSummaryVerbosity" });

// src/agent/openrouter/reasoning.ts
var AgentOpenrouterReasoningSchema = z.object({
  effort: AgentOpenrouterReasoningEffortSchema.nullable().describe("The reasoning effort level.\n\nOnly supported by some models.").meta({ omitempty: true }).optional(),
  enabled: z.boolean().nullable().describe("Whether reasoning is enabled. Defaults to `true` if other fields are set.").meta({ omitempty: true }).optional(),
  max_tokens: z.number().int().min(0).max(18446744073709552e3).nullable().describe("Maximum tokens for the reasoning/thinking output.\n\nOnly supported by some models.").meta({ omitempty: true }).optional(),
  summary_verbosity: AgentOpenrouterReasoningSummaryVerbositySchema.nullable().describe("Verbosity of reasoning summaries in the response.\n\nOnly supported by some models.").meta({ omitempty: true }).optional()
}).describe('Configuration for model reasoning/thinking capabilities.\n\nSome models (like o1, o3, Claude with extended thinking) support\nexplicit reasoning modes where they can "think" before responding.\nThis struct configures those capabilities.\n\n**Note:** The `max_tokens`, `effort`, and `summary_verbosity` fields are\nonly supported by some models. Unsupported fields are silently ignored.').meta({ title: "agent.openrouter.Reasoning" });
var AgentOpenrouterStopSchema = z.union([z.string().describe("A single stop sequence.").meta({ "variantTitle": "String" }), z.array(z.string()).describe("Multiple stop sequences (up to 4 typically supported).").meta({ "variantTitle": "Strings" })]).describe("Stop sequences that terminate model generation.\n\nWhen the model generates any of these sequences, it immediately\nstops producing further tokens.").meta({ title: "agent.openrouter.Stop" });
var AgentOpenrouterUpstreamSchema = z.literal("openrouter").describe("OpenRouter upstream marker.").meta({ title: "agent.openrouter.Upstream" });
var AgentOpenrouterVerbositySchema = z.union([z.literal("low").describe("Minimal output, concise responses.").meta({ "variantTitle": "Low" }), z.literal("medium").describe("Balanced output (default, normalized away during preparation).").meta({ "variantTitle": "Medium" }), z.literal("high").describe("Detailed output with thorough explanations.").meta({ "variantTitle": "High" }), z.literal("max").describe("Maximum verbosity, most detailed output possible.").meta({ "variantTitle": "Max" })]).describe("The verbosity level for model output.\n\nThis setting hints to the model how detailed its responses should be.\nNot all models support this parameter.").meta({ title: "agent.openrouter.Verbosity" });

// src/agent/openrouter/agentBase.ts
var AgentOpenrouterAgentBaseSchema = z.object({
  client_objectiveai_mcp: AgentClientObjectiveaiMcpSchema.nullable().describe("Client-side ObjectiveAI MCP surface the calling client is\nexpected to expose locally back to the API (objectiveai\nbuilt-in, plus specific plugins / tools by owner+name+version).").meta({ omitempty: true }).optional(),
  context_compression: AgentOpenrouterContextCompressionSchema.nullable().describe("Context compression engine for long contexts. When set, the\nupstream client emits the matching `plugins` entry on the\noutgoing OpenRouter chat-completions request.").meta({ omitempty: true }).optional(),
  frequency_penalty: z.number().min(-34028234663852886e22).max(34028234663852886e22).nullable().describe("Penalizes tokens based on their frequency in the output so far (-2.0 to 2.0).").meta({ omitempty: true }).optional(),
  logit_bias: z.record(z.string(), z.number().int().min(-9223372036854776e3).max(9223372036854776e3)).nullable().describe("Token ID to bias mapping (-100 to 100). Positive values increase likelihood.").meta({ omitempty: true }).optional(),
  max_completion_tokens: z.number().int().min(0).max(18446744073709552e3).nullable().describe("Maximum tokens in the completion.").meta({ omitempty: true }).optional(),
  max_tokens: z.number().int().min(0).max(18446744073709552e3).nullable().describe("Maximum tokens (OpenRouter variant of max_completion_tokens).").meta({ omitempty: true }).optional(),
  mcp_servers: z.array(AgentMcpServerSchema).nullable().describe("MCP servers the agent can connect to.").meta({ omitempty: true }).optional(),
  min_p: z.number().min(-34028234663852886e22).max(34028234663852886e22).nullable().describe("Minimum probability threshold for sampling (0.0 to 1.0).").meta({ omitempty: true }).optional(),
  model: z.string().describe('The upstream language model identifier (e.g., `"gpt-4"`, `"claude-3-opus"`).'),
  output_mode: AgentOpenrouterOutputModeSchema.default("instruction").describe("The output mode for vector completions. Ignored for agent completions."),
  post_system_prefix_messages: z.array(AgentCompletionsMessageMessageSchema).nullable().describe("Messages inserted after the leading chain of system/developer messages.").meta({ omitempty: true }).optional(),
  prefix_messages: z.array(AgentCompletionsMessageMessageSchema).nullable().describe("Messages prepended to the user's prompt.").meta({ omitempty: true }).optional(),
  presence_penalty: z.number().min(-34028234663852886e22).max(34028234663852886e22).nullable().describe("Penalizes tokens based on their presence in the output so far (-2.0 to 2.0).").meta({ omitempty: true }).optional(),
  provider: AgentOpenrouterProviderSchema.nullable().describe("Provider routing preferences.").meta({ omitempty: true }).optional(),
  reasoning: AgentOpenrouterReasoningSchema.nullable().describe("Reasoning/thinking configuration for supported models.").meta({ omitempty: true }).optional(),
  repetition_penalty: z.number().min(-34028234663852886e22).max(34028234663852886e22).nullable().describe("Repetition penalty (0.0 to 2.0). Values > 1.0 penalize repetition.").meta({ omitempty: true }).optional(),
  stop: AgentOpenrouterStopSchema.nullable().describe("Stop sequences that halt generation.").meta({ omitempty: true }).optional(),
  suffix_messages: z.array(AgentCompletionsMessageMessageSchema).nullable().describe("Messages appended after the user's prompt.").meta({ omitempty: true }).optional(),
  synthetic_reasoning: z.boolean().nullable().describe("Enable synthetic reasoning for non-reasoning LLMs.\n\n**Vector completions only.** Ignored for agent completions.").meta({ omitempty: true }).optional(),
  temperature: z.number().min(-34028234663852886e22).max(34028234663852886e22).nullable().describe("Sampling temperature (0.0 to 2.0). Higher = more random.").meta({ omitempty: true }).optional(),
  top_a: z.number().min(-34028234663852886e22).max(34028234663852886e22).nullable().describe("Top-a sampling parameter (0.0 to 1.0).").meta({ omitempty: true }).optional(),
  top_k: z.number().int().min(0).max(18446744073709552e3).nullable().describe("Top-k sampling: only consider the k most likely tokens.").meta({ omitempty: true }).optional(),
  top_logprobs: z.number().int().min(0).max(18446744073709552e3).nullable().describe("Number of top log probabilities to return (2-20).\n\n**Vector completions only.** Ignored for agent completions.").meta({ omitempty: true }).optional(),
  top_p: z.number().min(-34028234663852886e22).max(34028234663852886e22).nullable().describe("Nucleus sampling probability (0.0 to 1.0).").meta({ omitempty: true }).optional(),
  upstream: AgentOpenrouterUpstreamSchema.describe("The upstream provider marker."),
  verbosity: AgentOpenrouterVerbositySchema.nullable().describe("Output verbosity hint for supported models.").meta({ omitempty: true }).optional()
}).describe("The base configuration for an OpenRouter Agent (without computed ID).").meta({ title: "agent.openrouter.AgentBase" });

// src/agent/inlineAgentBase.ts
var AgentInlineAgentBaseSchema = z.union([AgentOpenrouterAgentBaseSchema.meta({ "title": "agent.openrouter.AgentBase", "variantTitle": "Openrouter" }), AgentClaudeAgentSdkAgentBaseSchema.meta({ "title": "agent.claude_agent_sdk.AgentBase", "variantTitle": "ClaudeAgentSdk" }), AgentCodexSdkAgentBaseSchema.meta({ "title": "agent.codex_sdk.AgentBase", "variantTitle": "CodexSdk" }), AgentMockAgentBaseSchema.meta({ "title": "agent.mock.AgentBase", "variantTitle": "Mock" })]).describe("The base inline configuration for an Agent (without computed ID or metadata).\n\nThis is an untagged enum that dispatches to the per-upstream AgentBase.\nDeserialization tries each variant in order until one matches.").meta({ title: "agent.InlineAgentBase" });

// src/agent/inlineAgentBaseWithFallbacks.ts
var AgentInlineAgentBaseWithFallbacksSchema = AgentInlineAgentBaseSchema.and(z.object({
  fallbacks: z.array(AgentInlineAgentBaseSchema).nullable().describe("Fallback agents to try if the primary fails.").meta({ omitempty: true }).optional()
})).describe("An [`InlineAgentBase`] with optional fallbacks (no description).").meta({ title: "agent.InlineAgentBaseWithFallbacks" });
var RemotePathCommitOptionalSchema = z.union([z.object({
  commit: z.string().nullable().optional(),
  owner: z.string(),
  remote: z.literal("github"),
  repository: z.string()
}).meta({ "variantTitle": "Github" }), z.object({
  commit: z.string().nullable().optional(),
  owner: z.string(),
  remote: z.literal("client"),
  repository: z.string()
}).meta({ "variantTitle": "Client" }), z.object({
  name: z.string(),
  remote: z.literal("mock")
}).meta({ "variantTitle": "Mock" })]).meta({ title: "RemotePathCommitOptional" });

// src/agent/inlineAgentBaseWithFallbacksOrRemoteCommitOptional.ts
var AgentInlineAgentBaseWithFallbacksOrRemoteCommitOptionalSchema = z.union([AgentInlineAgentBaseWithFallbacksSchema.meta({ "title": "agent.InlineAgentBaseWithFallbacks", "variantTitle": "AgentBase" }), RemotePathCommitOptionalSchema.meta({ "title": "RemotePathCommitOptional", "variantTitle": "Remote" })]).describe("Like [`InlineAgentBaseWithFallbacksOrRemote`] but with optional commit.\nUsed in request types where commit resolution happens server-side.").meta({ title: "agent.InlineAgentBaseWithFallbacksOrRemoteCommitOptional" });

// src/agent/completions/request/agentCompletionCreateParams.ts
var AgentCompletionsRequestAgentCompletionCreateParamsSchema = z.object({
  agent: AgentInlineAgentBaseWithFallbacksOrRemoteCommitOptionalSchema.describe("The agent to use (inline Agent or stored ID)."),
  continuation: z.string().nullable().describe("Continuation from a previous completion, as a base64-encoded string.").meta({ omitempty: true }).optional(),
  messages: z.array(AgentCompletionsMessageMessageSchema).describe("The conversation messages."),
  provider: AgentCompletionsRequestProviderSchema.nullable().describe("Provider routing preferences.").meta({ omitempty: true }).optional(),
  response_format: AgentCompletionsRequestResponseFormatParamSchema.nullable().describe("Output format constraints (text, JSON, or JSON schema).").meta({ omitempty: true }).optional(),
  seed: z.number().int().min(-9223372036854776e3).max(9223372036854776e3).nullable().describe("Random seed for deterministic generation.").meta({ omitempty: true }).optional(),
  stream: z.boolean().nullable().describe("Whether to stream the response.").meta({ omitempty: true }).optional()
}).describe("Parameters for creating a agent completion.").meta({ title: "agent.completions.request.AgentCompletionCreateParams" });
var FunctionsExecutionsRequestReasoningSchema = z.object({
  agent: AgentInlineAgentBaseWithFallbacksOrRemoteCommitOptionalSchema
}).meta({ title: "functions.executions.request.Reasoning" });
var FunctionsExecutionsRequestStrategySchema = z.union([z.object({
  type: z.literal("default")
}).describe("Scalar or Vector").meta({ "variantTitle": "Default" }), z.object({
  pool: z.number().int().min(0).max(4294967295).nullable().describe("How many vector responses for each execution").optional(),
  rounds: z.number().int().min(0).max(4294967295).nullable().describe("How many sequential rounds of comparison").optional(),
  type: z.literal("swiss_system")
}).describe("Vector").meta({ "variantTitle": "SwissSystem" })]).meta({ title: "functions.executions.request.Strategy" });
var FunctionsExpressionInputValueSchema = z.union([AgentCompletionsMessageRichContentPartSchema.describe("Rich content (image, audio, video, file).").meta({ "title": "agent.completions.message.RichContentPart", "variantTitle": "RichContentPart" }), z.record(z.string(), z.lazy(() => FunctionsExpressionInputValueSchema)).describe("An object with string keys.").meta({ "variantTitle": "Object" }), z.array(z.lazy(() => FunctionsExpressionInputValueSchema)).describe("An array of values.").meta({ "variantTitle": "Array" }), z.string().describe("A string value.").meta({ "variantTitle": "String" }), z.number().int().min(-9223372036854776e3).max(9223372036854776e3).describe("An integer value.").meta({ "variantTitle": "Integer" }), z.number().min(-34028234663852886e22).max(34028234663852886e22).describe("A floating-point number.").meta({ "variantTitle": "Number" }), z.boolean().describe("A boolean value.").meta({ "variantTitle": "Boolean" })]).describe("A concrete input value (post-compilation).\n\nRepresents any JSON-like value that can be passed to a Function,\nincluding rich content types (images, audio, video, files).").meta({ title: "functions.expression.InputValue" });
var FunctionsExpressionSpecialSchema = z.union([z.literal("input").describe("Returns the params input as-is.").meta({ "variantTitle": "Input" }), z.literal("output").describe("Returns the params output as-is.").meta({ "variantTitle": "Output" }), z.literal("task_output_l1_normalized").describe("L1-normalizes the output. Scalar/Err pass through.\nVector: L1 normalize. Vectors: L1 normalize each.").meta({ "variantTitle": "TaskOutputL1Normalized" }), z.literal("task_output_weighted_sum").describe("Weighted sum of the output. Vector \u2192 Scalar. Vectors \u2192 Vector.").meta({ "variantTitle": "TaskOutputWeightedSum" }), z.literal("input_items_output_length").describe("Returns the length of input['items'] as u64").meta({ "variantTitle": "InputItemsOutputLength" }), z.literal("input_items_optional_context_split").describe("Splits an input containing items and optionally context into multiple inputs").meta({ "variantTitle": "InputItemsOptionalContextSplit" }), z.literal("input_items_optional_context_merge").describe("Merges multiple inputs containing items and optionally context into a single input").meta({ "variantTitle": "InputItemsOptionalContextMerge" })]).describe("Predefined expression behaviors that require no user-authored code.").meta({ title: "functions.expression.Special" });

// src/functions/expression/expression.ts
var FunctionsExpressionExpressionSchema = z.union([z.object({
  $jmespath: z.string()
}).strict().describe("A JMESPath expression.").meta({ "variantTitle": "JMESPath" }), z.object({
  $starlark: z.string()
}).strict().describe("A Starlark expression.").meta({ "variantTitle": "Starlark" }), z.object({
  $special: FunctionsExpressionSpecialSchema
}).strict().describe("A predefined special expression variant.").meta({ "variantTitle": "Special" })]).describe('An expression that can be either JMESPath or Starlark.\n\nSerializes as `{"$jmespath": "..."}` or `{"$starlark": "..."}` in JSON.\n\n# Examples\n\nJMESPath:\n```json\n{"$jmespath": "input.items[0].name"}\n```\n\nStarlark:\n```json\n{"$starlark": "input[\'items\'][0][\'name\']"}\n```').meta({ title: "functions.expression.Expression" });
var FunctionsExpressionAnyOfInputSchemaSchema = z.object({
  anyOf: z.array(z.lazy(() => FunctionsExpressionInputSchemaSchema)).describe("The possible schemas that the input can match.")
}).describe("Schema for a union of possible types - input must match at least one.").meta({ title: "functions.expression.AnyOfInputSchema" });
var FunctionsExpressionArrayInputSchemaTypeSchema = z.literal("array").meta({ title: "functions.expression.ArrayInputSchemaType" });

// src/functions/expression/arrayInputSchema.ts
var FunctionsExpressionArrayInputSchemaSchema = z.object({
  description: z.string().nullable().describe("Human-readable description of the array.").meta({ omitempty: true }).optional(),
  items: z.lazy(() => FunctionsExpressionInputSchemaSchema).describe("Schema for each item in the array."),
  maxItems: z.number().int().min(0).max(18446744073709552e3).nullable().describe("Maximum number of items allowed.").meta({ omitempty: true }).optional(),
  minItems: z.number().int().min(0).max(18446744073709552e3).nullable().describe("Minimum number of items required.").meta({ omitempty: true }).optional(),
  type: FunctionsExpressionArrayInputSchemaTypeSchema
}).describe("Schema for an array input.").meta({ title: "functions.expression.ArrayInputSchema" });
var FunctionsExpressionAudioInputSchemaTypeSchema = z.literal("audio").meta({ title: "functions.expression.AudioInputSchemaType" });

// src/functions/expression/audioInputSchema.ts
var FunctionsExpressionAudioInputSchemaSchema = z.object({
  description: z.string().nullable().describe("Human-readable description of the expected audio.").meta({ omitempty: true }).optional(),
  type: FunctionsExpressionAudioInputSchemaTypeSchema
}).describe("Schema for an audio input.").meta({ title: "functions.expression.AudioInputSchema" });
var FunctionsExpressionBooleanInputSchemaTypeSchema = z.literal("boolean").meta({ title: "functions.expression.BooleanInputSchemaType" });

// src/functions/expression/booleanInputSchema.ts
var FunctionsExpressionBooleanInputSchemaSchema = z.object({
  description: z.string().nullable().describe("Human-readable description of the boolean.").meta({ omitempty: true }).optional(),
  type: FunctionsExpressionBooleanInputSchemaTypeSchema
}).describe("Schema for a boolean input.").meta({ title: "functions.expression.BooleanInputSchema" });
var FunctionsExpressionFileInputSchemaTypeSchema = z.literal("file").meta({ title: "functions.expression.FileInputSchemaType" });

// src/functions/expression/fileInputSchema.ts
var FunctionsExpressionFileInputSchemaSchema = z.object({
  description: z.string().nullable().describe("Human-readable description of the expected file.").meta({ omitempty: true }).optional(),
  type: FunctionsExpressionFileInputSchemaTypeSchema
}).describe("Schema for a file input.").meta({ title: "functions.expression.FileInputSchema" });
var FunctionsExpressionImageInputSchemaTypeSchema = z.literal("image").meta({ title: "functions.expression.ImageInputSchemaType" });

// src/functions/expression/imageInputSchema.ts
var FunctionsExpressionImageInputSchemaSchema = z.object({
  description: z.string().nullable().describe("Human-readable description of the expected image.").meta({ omitempty: true }).optional(),
  type: FunctionsExpressionImageInputSchemaTypeSchema
}).describe("Schema for an image input (URL or base64-encoded).").meta({ title: "functions.expression.ImageInputSchema" });
var FunctionsExpressionIntegerInputSchemaTypeSchema = z.literal("integer").meta({ title: "functions.expression.IntegerInputSchemaType" });

// src/functions/expression/integerInputSchema.ts
var FunctionsExpressionIntegerInputSchemaSchema = z.object({
  description: z.string().nullable().describe("Human-readable description of the integer.").meta({ omitempty: true }).optional(),
  maximum: z.number().int().min(-9223372036854776e3).max(9223372036854776e3).nullable().describe("Maximum allowed value (inclusive).").meta({ omitempty: true }).optional(),
  minimum: z.number().int().min(-9223372036854776e3).max(9223372036854776e3).nullable().describe("Minimum allowed value (inclusive).").meta({ omitempty: true }).optional(),
  type: FunctionsExpressionIntegerInputSchemaTypeSchema
}).describe("Schema for an integer input.").meta({ title: "functions.expression.IntegerInputSchema" });
var FunctionsExpressionNumberInputSchemaTypeSchema = z.literal("number").meta({ title: "functions.expression.NumberInputSchemaType" });

// src/functions/expression/numberInputSchema.ts
var FunctionsExpressionNumberInputSchemaSchema = z.object({
  description: z.string().nullable().describe("Human-readable description of the number.").meta({ omitempty: true }).optional(),
  maximum: z.number().min(-34028234663852886e22).max(34028234663852886e22).nullable().describe("Maximum allowed value (inclusive).").meta({ omitempty: true }).optional(),
  minimum: z.number().min(-34028234663852886e22).max(34028234663852886e22).nullable().describe("Minimum allowed value (inclusive).").meta({ omitempty: true }).optional(),
  type: FunctionsExpressionNumberInputSchemaTypeSchema
}).describe("Schema for a floating-point number input.").meta({ title: "functions.expression.NumberInputSchema" });
var FunctionsExpressionStringInputSchemaTypeSchema = z.literal("string").meta({ title: "functions.expression.StringInputSchemaType" });

// src/functions/expression/stringInputSchema.ts
var FunctionsExpressionStringInputSchemaSchema = z.object({
  description: z.string().nullable().describe("Human-readable description of the string.").meta({ omitempty: true }).optional(),
  enum: z.array(z.string()).nullable().describe("If provided, the string must be one of these values.").meta({ omitempty: true }).optional(),
  type: FunctionsExpressionStringInputSchemaTypeSchema
}).describe("Schema for a string input.").meta({ title: "functions.expression.StringInputSchema" });
var FunctionsExpressionVideoInputSchemaTypeSchema = z.literal("video").meta({ title: "functions.expression.VideoInputSchemaType" });

// src/functions/expression/videoInputSchema.ts
var FunctionsExpressionVideoInputSchemaSchema = z.object({
  description: z.string().nullable().describe("Human-readable description of the expected video.").meta({ omitempty: true }).optional(),
  type: FunctionsExpressionVideoInputSchemaTypeSchema
}).describe("Schema for a video input (URL or base64-encoded).").meta({ title: "functions.expression.VideoInputSchema" });

// src/functions/expression/inputSchema.ts
var FunctionsExpressionInputSchemaSchema = z.union([z.lazy(() => FunctionsExpressionAnyOfInputSchemaSchema).describe("A union of schemas - input must match at least one.").meta({ "title": "functions.expression.AnyOfInputSchema", "variantTitle": "AnyOf" }), z.lazy(() => FunctionsExpressionObjectInputSchemaSchema).describe("An object with named properties.").meta({ "title": "functions.expression.ObjectInputSchema", "variantTitle": "Object" }), z.lazy(() => FunctionsExpressionArrayInputSchemaSchema).describe("An array of items.").meta({ "title": "functions.expression.ArrayInputSchema", "variantTitle": "Array" }), FunctionsExpressionStringInputSchemaSchema.describe("A string value.").meta({ "title": "functions.expression.StringInputSchema", "variantTitle": "String" }), FunctionsExpressionIntegerInputSchemaSchema.describe("An integer value.").meta({ "title": "functions.expression.IntegerInputSchema", "variantTitle": "Integer" }), FunctionsExpressionNumberInputSchemaSchema.describe("A floating-point number.").meta({ "title": "functions.expression.NumberInputSchema", "variantTitle": "Number" }), FunctionsExpressionBooleanInputSchemaSchema.describe("A boolean value.").meta({ "title": "functions.expression.BooleanInputSchema", "variantTitle": "Boolean" }), FunctionsExpressionImageInputSchemaSchema.describe("An image (URL or base64).").meta({ "title": "functions.expression.ImageInputSchema", "variantTitle": "Image" }), FunctionsExpressionAudioInputSchemaSchema.describe("Audio content.").meta({ "title": "functions.expression.AudioInputSchema", "variantTitle": "Audio" }), FunctionsExpressionVideoInputSchemaSchema.describe("Video content.").meta({ "title": "functions.expression.VideoInputSchema", "variantTitle": "Video" }), FunctionsExpressionFileInputSchemaSchema.describe("A file.").meta({ "title": "functions.expression.FileInputSchema", "variantTitle": "File" })]).describe("Schema for validating Function input.\n\nDefines the expected structure and constraints for input data.\nUsed by remote Functions to document and validate their inputs.").meta({ title: "functions.expression.InputSchema" });
var FunctionsExpressionObjectInputSchemaTypeSchema = z.literal("object").meta({ title: "functions.expression.ObjectInputSchemaType" });

// src/functions/expression/objectInputSchema.ts
var FunctionsExpressionObjectInputSchemaSchema = z.object({
  description: z.string().nullable().describe("Human-readable description of the object.").meta({ omitempty: true }).optional(),
  properties: z.record(z.string(), z.lazy(() => FunctionsExpressionInputSchemaSchema)).describe("Schema for each property in the object."),
  required: z.array(z.string()).nullable().describe("List of property names that must be present.").meta({ omitempty: true }).optional(),
  type: FunctionsExpressionObjectInputSchemaTypeSchema
}).describe("Schema for an object input with named properties.").meta({ title: "functions.expression.ObjectInputSchema" });

// src/functions/alpha_scalar/placeholderScalarFunctionTaskExpression.ts
var FunctionsAlphaScalarPlaceholderScalarFunctionTaskExpressionSchema = z.object({
  input: FunctionsExpressionExpressionSchema,
  input_schema: FunctionsExpressionObjectInputSchemaSchema,
  skip: FunctionsExpressionExpressionSchema.nullable().meta({ omitempty: true }).optional()
}).meta({ title: "functions.alpha_scalar.PlaceholderScalarFunctionTaskExpression" });
var FunctionsAlphaScalarScalarFunctionTaskExpressionSchema = RemotePathSchema.and(z.object({
  input: FunctionsExpressionExpressionSchema,
  skip: FunctionsExpressionExpressionSchema.nullable().meta({ omitempty: true }).optional()
})).meta({ title: "functions.alpha_scalar.ScalarFunctionTaskExpression" });

// src/functions/alpha_scalar/branchTaskExpression.ts
var FunctionsAlphaScalarBranchTaskExpressionSchema = z.union([FunctionsAlphaScalarScalarFunctionTaskExpressionSchema.and(z.object({
  type: z.literal("alpha.scalar.function")
})).meta({ "variantTitle": "ScalarFunction" }), FunctionsAlphaScalarPlaceholderScalarFunctionTaskExpressionSchema.and(z.object({
  type: z.literal("placeholder.alpha.scalar.function")
})).meta({ "variantTitle": "PlaceholderScalarFunction" })]).meta({ title: "functions.alpha_scalar.BranchTaskExpression" });
var FunctionsAlphaScalarVectorCompletionTaskExpressionSchema = z.object({
  messages: FunctionsExpressionExpressionSchema,
  responses: z.array(AgentCompletionsMessageRichContentSchema),
  skip: FunctionsExpressionExpressionSchema.nullable().meta({ omitempty: true }).optional()
}).meta({ title: "functions.alpha_scalar.VectorCompletionTaskExpression" });

// src/functions/alpha_scalar/leafTaskExpression.ts
var FunctionsAlphaScalarLeafTaskExpressionSchema = FunctionsAlphaScalarVectorCompletionTaskExpressionSchema.and(z.object({
  type: z.literal("vector.completion")
})).meta({ title: "functions.alpha_scalar.LeafTaskExpression" });

// src/functions/alpha_scalar/inlineFunction.ts
var FunctionsAlphaScalarInlineFunctionSchema = z.union([z.object({
  tasks: z.array(FunctionsAlphaScalarBranchTaskExpressionSchema),
  type: z.literal("alpha.scalar.branch.function")
}).meta({ "variantTitle": "Branch" }), z.object({
  tasks: z.array(FunctionsAlphaScalarLeafTaskExpressionSchema),
  type: z.literal("alpha.scalar.leaf.function")
}).meta({ "variantTitle": "Leaf" })]).meta({ title: "functions.alpha_scalar.InlineFunction" });
var FunctionsAlphaVectorPlaceholderScalarFunctionTaskExpressionSchema = z.object({
  input: FunctionsExpressionExpressionSchema,
  input_schema: FunctionsExpressionObjectInputSchemaSchema,
  skip: FunctionsExpressionExpressionSchema.nullable().meta({ omitempty: true }).optional()
}).meta({ title: "functions.alpha_vector.PlaceholderScalarFunctionTaskExpression" });
var FunctionsAlphaVectorExpressionVectorFunctionInputSchemaSchema = z.object({
  context: FunctionsExpressionObjectInputSchemaSchema.nullable().meta({ omitempty: true }).optional(),
  items: FunctionsExpressionInputSchemaSchema
}).meta({ title: "functions.alpha_vector.expression.VectorFunctionInputSchema" });
var FunctionsAlphaVectorExpressionVectorFunctionInputValueExpressionSchema = z.object({
  context: FunctionsExpressionExpressionSchema.nullable().meta({ omitempty: true }).optional(),
  items: FunctionsExpressionExpressionSchema
}).meta({ title: "functions.alpha_vector.expression.VectorFunctionInputValueExpression" });

// src/functions/alpha_vector/placeholderVectorFunctionTaskExpression.ts
var FunctionsAlphaVectorPlaceholderVectorFunctionTaskExpressionSchema = z.object({
  input: FunctionsAlphaVectorExpressionVectorFunctionInputValueExpressionSchema,
  input_schema: FunctionsAlphaVectorExpressionVectorFunctionInputSchemaSchema,
  skip: FunctionsExpressionExpressionSchema.nullable().meta({ omitempty: true }).optional()
}).meta({ title: "functions.alpha_vector.PlaceholderVectorFunctionTaskExpression" });
var FunctionsAlphaVectorScalarFunctionTaskExpressionSchema = RemotePathSchema.and(z.object({
  input: FunctionsExpressionExpressionSchema,
  skip: FunctionsExpressionExpressionSchema.nullable().meta({ omitempty: true }).optional()
})).meta({ title: "functions.alpha_vector.ScalarFunctionTaskExpression" });
var FunctionsAlphaVectorVectorFunctionTaskExpressionSchema = RemotePathSchema.and(z.object({
  input: FunctionsAlphaVectorExpressionVectorFunctionInputValueExpressionSchema,
  skip: FunctionsExpressionExpressionSchema.nullable().meta({ omitempty: true }).optional()
})).meta({ title: "functions.alpha_vector.VectorFunctionTaskExpression" });

// src/functions/alpha_vector/branchTaskExpression.ts
var FunctionsAlphaVectorBranchTaskExpressionSchema = z.union([FunctionsAlphaVectorScalarFunctionTaskExpressionSchema.and(z.object({
  type: z.literal("alpha.scalar.function")
})).meta({ "variantTitle": "ScalarFunction" }), FunctionsAlphaVectorVectorFunctionTaskExpressionSchema.and(z.object({
  type: z.literal("alpha.vector.function")
})).meta({ "variantTitle": "VectorFunction" }), FunctionsAlphaVectorPlaceholderScalarFunctionTaskExpressionSchema.and(z.object({
  type: z.literal("placeholder.alpha.scalar.function")
})).meta({ "variantTitle": "PlaceholderScalarFunction" }), FunctionsAlphaVectorPlaceholderVectorFunctionTaskExpressionSchema.and(z.object({
  type: z.literal("placeholder.alpha.vector.function")
})).meta({ "variantTitle": "PlaceholderVectorFunction" })]).meta({ title: "functions.alpha_vector.BranchTaskExpression" });
var FunctionsAlphaVectorVectorCompletionTaskExpressionSchema = z.object({
  messages: FunctionsExpressionExpressionSchema,
  responses: FunctionsExpressionExpressionSchema,
  skip: FunctionsExpressionExpressionSchema.nullable().meta({ omitempty: true }).optional()
}).meta({ title: "functions.alpha_vector.VectorCompletionTaskExpression" });

// src/functions/alpha_vector/leafTaskExpression.ts
var FunctionsAlphaVectorLeafTaskExpressionSchema = FunctionsAlphaVectorVectorCompletionTaskExpressionSchema.and(z.object({
  type: z.literal("vector.completion")
})).meta({ title: "functions.alpha_vector.LeafTaskExpression" });

// src/functions/alpha_vector/inlineFunction.ts
var FunctionsAlphaVectorInlineFunctionSchema = z.union([z.object({
  tasks: z.array(FunctionsAlphaVectorBranchTaskExpressionSchema),
  type: z.literal("alpha.vector.branch.function")
}).meta({ "variantTitle": "Branch" }), z.object({
  tasks: z.array(FunctionsAlphaVectorLeafTaskExpressionSchema),
  type: z.literal("alpha.vector.leaf.function")
}).meta({ "variantTitle": "Leaf" })]).meta({ title: "functions.alpha_vector.InlineFunction" });

// src/functions/alphaInlineFunction.ts
var FunctionsAlphaInlineFunctionSchema = z.union([FunctionsAlphaScalarInlineFunctionSchema.meta({ "title": "functions.alpha_scalar.InlineFunction", "variantTitle": "Scalar" }), FunctionsAlphaVectorInlineFunctionSchema.meta({ "title": "functions.alpha_vector.InlineFunction", "variantTitle": "Vector" })]).meta({ title: "functions.AlphaInlineFunction" });
var FunctionsExpressionInputValueExpressionSchema = z.union([AgentCompletionsMessageRichContentPartSchema.describe("Rich content (image, audio, video, file).").meta({ "title": "agent.completions.message.RichContentPart", "variantTitle": "RichContentPart" }), z.record(z.string(), z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), z.lazy(() => FunctionsExpressionInputValueExpressionSchema).describe("A literal value.").meta({ "title": "functions.expression.InputValueExpression", "variantTitle": "Value" })]).describe('A value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```')).describe("An object with values that may be expressions.").meta({ "variantTitle": "Object" }), z.array(z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), z.lazy(() => FunctionsExpressionInputValueExpressionSchema).describe("A literal value.").meta({ "title": "functions.expression.InputValueExpression", "variantTitle": "Value" })]).describe('A value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```')).describe("An array with elements that may be expressions.").meta({ "variantTitle": "Array" }), z.string().describe("A string value.").meta({ "variantTitle": "String" }), z.number().int().min(-9223372036854776e3).max(9223372036854776e3).describe("An integer value.").meta({ "variantTitle": "Integer" }), z.number().min(-34028234663852886e22).max(34028234663852886e22).describe("A floating-point number.").meta({ "variantTitle": "Number" }), z.boolean().describe("A boolean value.").meta({ "variantTitle": "Boolean" })]).describe("An input value that may contain expressions (pre-compilation).\n\nSimilar to [`InputValue`] but object values and array elements can be\nexpressions (JMESPath or Starlark) that are evaluated during compilation.").meta({ title: "functions.expression.InputValueExpression" });

// src/functions/placeholderScalarFunctionTaskExpression.ts
var FunctionsPlaceholderScalarFunctionTaskExpressionSchema = z.object({
  input: z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), FunctionsExpressionInputValueExpressionSchema.describe("A literal value.").meta({ "title": "functions.expression.InputValueExpression", "variantTitle": "Value" })]).describe('Expression for the input to pass to the placeholder function.\nReceives: `input`, `map` (if mapped).\n\nA value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```'),
  input_schema: FunctionsExpressionInputSchemaSchema.describe("JSON Schema defining the expected input structure."),
  map: FunctionsExpressionExpressionSchema.nullable().describe("Expression that evaluates to the number of mapped task instances.\nEach instance receives `map` as an integer index (0-based).").meta({ omitempty: true }).optional(),
  output: FunctionsExpressionExpressionSchema.describe("Expression to transform the fixed 0.5 output.\nReceives: `input`, `output` as `Scalar(0.5)`."),
  skip: FunctionsExpressionExpressionSchema.nullable().describe("If this expression evaluates to true, skip the task. Receives: `input`.").meta({ omitempty: true }).optional()
}).describe("Expression for a placeholder scalar function task (pre-compilation).\n\nLike [`ScalarFunctionTaskExpression`] but without owner/repository/commit.\nAlways produces a fixed output of 0.5.").meta({ title: "functions.PlaceholderScalarFunctionTaskExpression" });
var FunctionsPlaceholderVectorFunctionTaskExpressionSchema = z.object({
  input: z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), FunctionsExpressionInputValueExpressionSchema.describe("A literal value.").meta({ "title": "functions.expression.InputValueExpression", "variantTitle": "Value" })]).describe('Expression for the input to pass to the placeholder function.\nReceives: `input`, `map` (if mapped).\n\nA value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```'),
  input_merge: FunctionsExpressionExpressionSchema.describe("Expression merging sub-inputs back into one input.\nReceives: `input` (as an array)."),
  input_schema: FunctionsExpressionInputSchemaSchema.describe("JSON Schema defining the expected input structure."),
  input_split: FunctionsExpressionExpressionSchema.describe("Expression transforming input into sub-inputs for swiss system.\nReceives: `input`."),
  map: FunctionsExpressionExpressionSchema.nullable().describe("Expression that evaluates to the number of mapped task instances.\nEach instance receives `map` as an integer index (0-based).").meta({ omitempty: true }).optional(),
  output: FunctionsExpressionExpressionSchema.describe("Expression to transform the equalized vector output.\nReceives: `input`, `output` as `Vector(equalized)`."),
  output_length: FunctionsExpressionExpressionSchema.describe("Expression computing the expected output vector length.\nReceives: `input`."),
  skip: FunctionsExpressionExpressionSchema.nullable().describe("If this expression evaluates to true, skip the task. Receives: `input`.").meta({ omitempty: true }).optional()
}).describe("Expression for a placeholder vector function task (pre-compilation).\n\nLike [`VectorFunctionTaskExpression`] but without owner/repository/commit.\nAlways produces an equalized vector of length `output_length`.").meta({ title: "functions.PlaceholderVectorFunctionTaskExpression" });
var FunctionsScalarFunctionTaskExpressionSchema = RemotePathSchema.and(z.object({
  input: z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), FunctionsExpressionInputValueExpressionSchema.describe("A literal value.").meta({ "title": "functions.expression.InputValueExpression", "variantTitle": "Value" })]).describe('Expression for the input to pass to the function.\nReceives: `input`, `map` (if mapped).\n\nA value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```'),
  map: FunctionsExpressionExpressionSchema.nullable().describe("Expression that evaluates to the number of mapped task instances.\nEach instance receives `map` as an integer index (0-based).").meta({ omitempty: true }).optional(),
  output: FunctionsExpressionExpressionSchema.describe("Expression to transform the task result into a valid function output.\n\nReceives `output` which is one of 4 variants:\n- `Scalar(Decimal)` - a single score\n- `Vector(Vec<Decimal>)` - a vector of scores\n- `Vectors(Vec<Vec<Decimal>>)` - multiple vectors (from mapped tasks)\n- `Err(Value)` - an error\n\nThe expression must return a `TaskOutputOwned` that is valid for the parent function's type:\n- For scalar functions: must return `Scalar(value)` where value is in [0, 1]\n- For vector functions: must return `Vector(values)` where values sum to ~1 and match the expected length\n\nThe function's final output is computed as a weighted average of all task outputs using\nprofile weights. If a function has only one task, that task's output becomes the function's\noutput directly."),
  skip: FunctionsExpressionExpressionSchema.nullable().describe("If this expression evaluates to true, skip the task. Receives: `input`.").meta({ omitempty: true }).optional()
})).describe("Expression for a task that calls a scalar function (pre-compilation).").meta({ title: "functions.ScalarFunctionTaskExpression" });
var AgentCompletionsMessageAssistantToolCallFunctionExpressionSchema = z.object({
  arguments: z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), z.string().describe("A literal value.").meta({ "variantTitle": "Value" })]).describe('The arguments expression.\n\nA value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```'),
  name: z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), z.string().describe("A literal value.").meta({ "variantTitle": "Value" })]).describe('The function name expression.\n\nA value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```')
}).describe("Expression variant of [`AssistantToolCallFunction`] for dynamic content.").meta({ title: "agent.completions.message.AssistantToolCallFunctionExpression" });

// src/agent/completions/message/assistantToolCallExpression.ts
var AgentCompletionsMessageAssistantToolCallExpressionSchema = z.object({
  function: z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), AgentCompletionsMessageAssistantToolCallFunctionExpressionSchema.describe("A literal value.").meta({ "title": "agent.completions.message.AssistantToolCallFunctionExpression", "variantTitle": "Value" })]).describe('The function expression.\n\nA value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```'),
  id: z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), z.string().describe("A literal value.").meta({ "variantTitle": "Value" })]).describe('The tool call ID expression.\n\nA value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```'),
  type: z.literal("function")
}).describe("A function call expression.").meta({ title: "agent.completions.message.AssistantToolCallExpression" });
var AgentCompletionsMessageRichContentPartExpressionSchema = z.union([z.object({
  text: z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), z.string().describe("A literal value.").meta({ "variantTitle": "Value" })]).describe('A value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```'),
  type: z.literal("text")
}).meta({ "variantTitle": "Text" }), z.object({
  image_url: z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), AgentCompletionsMessageImageUrlSchema.describe("A literal value.").meta({ "title": "agent.completions.message.ImageUrl", "variantTitle": "Value" })]).describe('A value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```'),
  type: z.literal("image_url")
}).meta({ "variantTitle": "ImageUrl" }), z.object({
  input_audio: z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), AgentCompletionsMessageInputAudioSchema.describe("A literal value.").meta({ "title": "agent.completions.message.InputAudio", "variantTitle": "Value" })]).describe('A value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```'),
  type: z.literal("input_audio")
}).meta({ "variantTitle": "InputAudio" }), z.object({
  type: z.literal("input_video"),
  video_url: z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), AgentCompletionsMessageVideoUrlSchema.describe("A literal value.").meta({ "title": "agent.completions.message.VideoUrl", "variantTitle": "Value" })]).describe('A value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```')
}).meta({ "variantTitle": "InputVideo" }), z.object({
  type: z.literal("video_url"),
  video_url: z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), AgentCompletionsMessageVideoUrlSchema.describe("A literal value.").meta({ "title": "agent.completions.message.VideoUrl", "variantTitle": "Value" })]).describe('A value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```')
}).meta({ "variantTitle": "VideoUrl" }), z.object({
  file: z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), AgentCompletionsMessageFileSchema.describe("A literal value.").meta({ "title": "agent.completions.message.File", "variantTitle": "Value" })]).describe('A value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```'),
  type: z.literal("file")
}).meta({ "variantTitle": "File" })]).describe("Expression variant of [`RichContentPart`] for dynamic content.").meta({ title: "agent.completions.message.RichContentPartExpression" });

// src/agent/completions/message/richContentExpression.ts
var AgentCompletionsMessageRichContentExpressionSchema = z.union([z.string().describe("Plain text content.").meta({ "variantTitle": "Text" }), z.array(z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), AgentCompletionsMessageRichContentPartExpressionSchema.describe("A literal value.").meta({ "title": "agent.completions.message.RichContentPartExpression", "variantTitle": "Value" })]).describe('A value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```')).describe("Multi-part content expressions.").meta({ "variantTitle": "Parts" })]).describe("Expression variant of [`RichContent`] for dynamic content.").meta({ title: "agent.completions.message.RichContentExpression" });

// src/agent/completions/message/assistantMessageExpression.ts
var AgentCompletionsMessageAssistantMessageExpressionSchema = z.object({
  content: z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), AgentCompletionsMessageRichContentExpressionSchema.describe("A literal value.").meta({ "title": "agent.completions.message.RichContentExpression", "variantTitle": "Value" })]).describe('A value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```').nullable().describe("The content expression.").meta({ omitempty: true }).optional(),
  name: z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), z.string().describe("A literal value.").meta({ "variantTitle": "Value" })]).describe('A value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```').nullable().meta({ omitempty: true }).optional(),
  reasoning: z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), z.string().describe("A literal value.").meta({ "variantTitle": "Value" })]).describe('A value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```').nullable().meta({ omitempty: true }).optional(),
  refusal: z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), z.string().describe("A literal value.").meta({ "variantTitle": "Value" })]).describe('A value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```').nullable().meta({ omitempty: true }).optional(),
  tool_calls: z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), z.array(z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), AgentCompletionsMessageAssistantToolCallExpressionSchema.describe("A literal value.").meta({ "title": "agent.completions.message.AssistantToolCallExpression", "variantTitle": "Value" })]).describe('A value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```')).describe("A literal value.").meta({ "variantTitle": "Value" })]).describe('A value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```').nullable().meta({ omitempty: true }).optional()
}).describe("Expression variant of [`AssistantMessage`] for dynamic content.").meta({ title: "agent.completions.message.AssistantMessageExpression" });
var AgentCompletionsMessageSimpleContentPartExpressionSchema = z.object({
  text: z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), z.string().describe("A literal value.").meta({ "variantTitle": "Value" })]).describe('The text expression.\n\nA value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```'),
  type: z.literal("text")
}).describe("A text part expression.").meta({ title: "agent.completions.message.SimpleContentPartExpression" });

// src/agent/completions/message/simpleContentExpression.ts
var AgentCompletionsMessageSimpleContentExpressionSchema = z.union([z.string().describe("Plain text content.").meta({ "variantTitle": "Text" }), z.array(z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), AgentCompletionsMessageSimpleContentPartExpressionSchema.describe("A literal value.").meta({ "title": "agent.completions.message.SimpleContentPartExpression", "variantTitle": "Value" })]).describe('A value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```')).describe("Multi-part text content expressions.").meta({ "variantTitle": "Parts" })]).describe("Expression variant of [`SimpleContent`] for dynamic content.").meta({ title: "agent.completions.message.SimpleContentExpression" });

// src/agent/completions/message/developerMessageExpression.ts
var AgentCompletionsMessageDeveloperMessageExpressionSchema = z.object({
  content: z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), AgentCompletionsMessageSimpleContentExpressionSchema.describe("A literal value.").meta({ "title": "agent.completions.message.SimpleContentExpression", "variantTitle": "Value" })]).describe('The message content expression.\n\nA value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```'),
  name: z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), z.string().describe("A literal value.").meta({ "variantTitle": "Value" })]).describe('A value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```').nullable().describe("Optional name expression.").meta({ omitempty: true }).optional()
}).describe("Expression variant of [`DeveloperMessage`] for dynamic content.").meta({ title: "agent.completions.message.DeveloperMessageExpression" });
var AgentCompletionsMessageSystemMessageExpressionSchema = z.object({
  content: z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), AgentCompletionsMessageSimpleContentExpressionSchema.describe("A literal value.").meta({ "title": "agent.completions.message.SimpleContentExpression", "variantTitle": "Value" })]).describe('The message content expression.\n\nA value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```'),
  name: z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), z.string().describe("A literal value.").meta({ "variantTitle": "Value" })]).describe('A value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```').nullable().describe("Optional name expression.").meta({ omitempty: true }).optional()
}).describe("Expression variant of [`SystemMessage`] for dynamic content.").meta({ title: "agent.completions.message.SystemMessageExpression" });
var AgentCompletionsMessageToolMessageExpressionSchema = z.object({
  content: z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), AgentCompletionsMessageRichContentExpressionSchema.describe("A literal value.").meta({ "title": "agent.completions.message.RichContentExpression", "variantTitle": "Value" })]).describe('The content expression.\n\nA value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```'),
  tool_call_id: z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), z.string().describe("A literal value.").meta({ "variantTitle": "Value" })]).describe('The tool call ID expression.\n\nA value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```')
}).describe("Expression variant of [`ToolMessage`] for dynamic content.").meta({ title: "agent.completions.message.ToolMessageExpression" });
var AgentCompletionsMessageUserMessageExpressionSchema = z.object({
  content: z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), AgentCompletionsMessageRichContentExpressionSchema.describe("A literal value.").meta({ "title": "agent.completions.message.RichContentExpression", "variantTitle": "Value" })]).describe('The message content expression.\n\nA value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```'),
  name: z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), z.string().describe("A literal value.").meta({ "variantTitle": "Value" })]).describe('A value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```').nullable().describe("Optional name expression.").meta({ omitempty: true }).optional()
}).describe("Expression variant of [`UserMessage`] for dynamic content.").meta({ title: "agent.completions.message.UserMessageExpression" });

// src/agent/completions/message/messageExpression.ts
var AgentCompletionsMessageMessageExpressionSchema = z.union([AgentCompletionsMessageDeveloperMessageExpressionSchema.and(z.object({
  role: z.literal("developer")
})).meta({ "variantTitle": "Developer" }), AgentCompletionsMessageSystemMessageExpressionSchema.and(z.object({
  role: z.literal("system")
})).meta({ "variantTitle": "System" }), AgentCompletionsMessageUserMessageExpressionSchema.and(z.object({
  role: z.literal("user")
})).meta({ "variantTitle": "User" }), AgentCompletionsMessageAssistantMessageExpressionSchema.and(z.object({
  role: z.literal("assistant")
})).meta({ "variantTitle": "Assistant" }), AgentCompletionsMessageToolMessageExpressionSchema.and(z.object({
  role: z.literal("tool")
})).meta({ "variantTitle": "Tool" })]).describe("A message with expressions for dynamic content.\n\nThis is the expression variant of [`Message`] used in function definitions\nwhere message content can be computed from the function input at runtime.\nSupports both JMESPath and Starlark expressions.").meta({ title: "agent.completions.message.MessageExpression" });

// src/functions/vectorCompletionTaskExpression.ts
var FunctionsVectorCompletionTaskExpressionSchema = z.object({
  map: FunctionsExpressionExpressionSchema.nullable().describe("Expression that evaluates to the number of mapped task instances.\nEach instance receives `map` as an integer index (0-based).").meta({ omitempty: true }).optional(),
  messages: z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), z.array(z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), AgentCompletionsMessageMessageExpressionSchema.describe("A literal value.").meta({ "title": "agent.completions.message.MessageExpression", "variantTitle": "Value" })]).describe('A value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```')).describe("A literal value.").meta({ "variantTitle": "Value" })]).describe('Expression for the conversation messages (the prompt).\nReceives: `input`, `map` (if mapped).\n\nA value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```'),
  output: FunctionsExpressionExpressionSchema.describe("Expression to transform the task result into a valid function output.\n\nReceives `output` as the task's raw result (typically `Vector(scores)`).\n\nThe expression must return a `TaskOutputOwned` that is valid for the parent function's type:\n- For scalar functions: must return `Scalar(value)` where value is in [0, 1]\n- For vector functions: must return `Vector(values)` where values sum to ~1 and match the expected length\n\nThe function's final output is computed as a weighted average of all task outputs using\nprofile weights. If a function has only one task, that task's output becomes the function's\noutput directly."),
  responses: z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), z.array(z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), AgentCompletionsMessageRichContentExpressionSchema.describe("A literal value.").meta({ "title": "agent.completions.message.RichContentExpression", "variantTitle": "Value" })]).describe('A value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```')).describe("A literal value.").meta({ "variantTitle": "Value" })]).describe('Expression for the possible responses the LLMs can vote for.\nReceives: `input`, `map` (if mapped).\n\nA value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```'),
  skip: FunctionsExpressionExpressionSchema.nullable().describe("If this expression evaluates to true, skip the task. Receives: `input`.").meta({ omitempty: true }).optional()
}).describe("Expression for a task that runs a vector completion (pre-compilation).").meta({ title: "functions.VectorCompletionTaskExpression" });
var FunctionsVectorFunctionTaskExpressionSchema = RemotePathSchema.and(z.object({
  input: z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), FunctionsExpressionInputValueExpressionSchema.describe("A literal value.").meta({ "title": "functions.expression.InputValueExpression", "variantTitle": "Value" })]).describe('Expression for the input to pass to the function.\nReceives: `input`, `map` (if mapped).\n\nA value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```'),
  map: FunctionsExpressionExpressionSchema.nullable().describe("Expression that evaluates to the number of mapped task instances.\nEach instance receives `map` as an integer index (0-based).").meta({ omitempty: true }).optional(),
  output: FunctionsExpressionExpressionSchema.describe("Expression to transform the task result into a valid function output.\n\nReceives `output` which is one of 4 variants:\n- `Scalar(Decimal)` - a single score\n- `Vector(Vec<Decimal>)` - a vector of scores\n- `Vectors(Vec<Vec<Decimal>>)` - multiple vectors (from mapped tasks)\n- `Err(Value)` - an error\n\nThe expression must return a `TaskOutputOwned` that is valid for the parent function's type:\n- For scalar functions: must return `Scalar(value)` where value is in [0, 1]\n- For vector functions: must return `Vector(values)` where values sum to ~1 and match the expected length\n\nThe function's final output is computed as a weighted average of all task outputs using\nprofile weights. If a function has only one task, that task's output becomes the function's\noutput directly."),
  skip: FunctionsExpressionExpressionSchema.nullable().describe("If this expression evaluates to true, skip the task. Receives: `input`.").meta({ omitempty: true }).optional()
})).describe("Expression for a task that calls a vector function (pre-compilation).").meta({ title: "functions.VectorFunctionTaskExpression" });

// src/functions/taskExpression.ts
var FunctionsTaskExpressionSchema = z.union([FunctionsScalarFunctionTaskExpressionSchema.and(z.object({
  type: z.literal("scalar.function")
})).meta({ "variantTitle": "ScalarFunction" }), FunctionsVectorFunctionTaskExpressionSchema.and(z.object({
  type: z.literal("vector.function")
})).meta({ "variantTitle": "VectorFunction" }), FunctionsVectorCompletionTaskExpressionSchema.and(z.object({
  type: z.literal("vector.completion")
})).meta({ "variantTitle": "VectorCompletion" }), FunctionsPlaceholderScalarFunctionTaskExpressionSchema.and(z.object({
  type: z.literal("placeholder.scalar.function")
})).meta({ "variantTitle": "PlaceholderScalarFunction" }), FunctionsPlaceholderVectorFunctionTaskExpressionSchema.and(z.object({
  type: z.literal("placeholder.vector.function")
})).meta({ "variantTitle": "PlaceholderVectorFunction" })]).describe("A task definition with expressions (pre-compilation).\n\nTask expressions contain dynamic fields (JMESPath or Starlark) that are\nresolved against input data during compilation. Use [`compile`](Self::compile)\nto produce a concrete [`Task`].").meta({ title: "functions.TaskExpression" });

// src/functions/inlineFunction.ts
var FunctionsInlineFunctionSchema = z.union([z.object({
  tasks: z.array(FunctionsTaskExpressionSchema).describe("The list of tasks to execute. Tasks with a `map` expression are\nexpanded into multiple instances. Each instance is compiled with\n`map` set to the current integer index.\nReceives: `input`, `map` (if mapped)."),
  type: z.literal("scalar.function")
}).describe("Produces a single score in [0, 1].").meta({ "variantTitle": "Scalar" }), z.object({
  input_merge: FunctionsExpressionExpressionSchema.nullable().describe("Expression transforming an array of inputs computed by `input_split`\ninto a single Input object for the Function.\nReceives: `input` (as an array).\nOnly required if the request uses a strategy that needs input splitting.").optional(),
  input_split: FunctionsExpressionExpressionSchema.nullable().describe("Expression transforming input into an input array of the output_length\nWhen the Function is executed with any input from the array,\nThe output_length should be 1.\nReceives: `input`.\nOnly required if the request uses a strategy that needs input splitting.").optional(),
  tasks: z.array(FunctionsTaskExpressionSchema).describe("The list of tasks to execute. Tasks with a `map` expression are\nexpanded into multiple instances. Each instance is compiled with\n`map` set to the current integer index.\nReceives: `input`, `map` (if mapped)."),
  type: z.literal("vector.function")
}).describe("Produces a vector of scores that sums to 1.").meta({ "variantTitle": "Vector" })]).describe("An inline function definition without metadata.\n\nUsed when embedding function logic directly in requests rather than\nreferencing a remote function. Lacks description and input\nschema fields.").meta({ title: "functions.InlineFunction" });

// src/functions/fullInlineFunction.ts
var FunctionsFullInlineFunctionSchema = z.union([FunctionsAlphaInlineFunctionSchema.meta({ "title": "functions.AlphaInlineFunction", "variantTitle": "Alpha" }), FunctionsInlineFunctionSchema.meta({ "title": "functions.InlineFunction", "variantTitle": "Standard" })]).meta({ title: "functions.FullInlineFunction" });

// src/functions/fullInlineFunctionOrRemoteCommitOptional.ts
var FunctionsFullInlineFunctionOrRemoteCommitOptionalSchema = z.union([FunctionsFullInlineFunctionSchema.meta({ "title": "functions.FullInlineFunction", "variantTitle": "Inline" }), RemotePathCommitOptionalSchema.meta({ "title": "RemotePathCommitOptional", "variantTitle": "Remote" })]).describe("A function specification that is either a full inline function definition\nor a remote path reference.").meta({ title: "functions.FullInlineFunctionOrRemoteCommitOptional" });
var FunctionsTaskProfileSchema = z.union([RemotePathSchema.describe("Profile for a nested function task (references another profile).").meta({ "title": "RemotePath", "variantTitle": "Remote" }), z.lazy(() => FunctionsInlineProfileSchema).describe("Inline profile for a task (tasks-based or auto).").meta({ "title": "functions.InlineProfile", "variantTitle": "Inline" }), z.object({}).describe("Placeholder task \u2014 no configuration needed, output is fixed.").meta({ "variantTitle": "Placeholder" })]).describe("Configuration for a single task within a Profile.\n\nEach variant corresponds to a task type in the Function definition.").meta({ title: "functions.TaskProfile" });
var WeightsEntrySchema = z.object({
  invert: z.boolean().nullable().describe("If true, invert this agent's vote distribution before combining.\n\nWhen omitted or false, the vote distribution is used as-is.").meta({ omitempty: true }).optional(),
  weight: z.number().min(-34028234663852886e22).max(34028234663852886e22).describe("The weight for this agent in the swarm. Must be in [0, 1].")
}).describe("An entry in weights with an explicit weight and optional invert flag.").meta({ title: "WeightsEntry" });

// src/weights.ts
var WeightsSchema = z.union([z.array(z.number().min(-34028234663852886e22).max(34028234663852886e22)).describe("Simple vector of decimal weights.").meta({ "variantTitle": "Weights" }), z.array(WeightsEntrySchema).describe("Vector of entries with optional invert flags.").meta({ "variantTitle": "Entries" })]).describe("Weights for a swarm's agents.\n\n- `Weights(Vec<Decimal>)` - simple representation (no inversion)\n- `Entries(Vec<WeightsEntry>)` - weights with optional per-agent `invert`").meta({ title: "Weights" });

// src/functions/inlineTasksProfile.ts
var FunctionsInlineTasksProfileSchema = z.object({
  tasks: z.array(z.lazy(() => FunctionsTaskProfileSchema)).describe("Configuration for each task in the corresponding Function."),
  weights: WeightsSchema.nullable().describe("Optional weights for each Task in the corresponding Function.\nIf `None`, uniform weights are used.").meta({ omitempty: true }).optional()
}).describe("An inline tasks-based profile definition without metadata.").meta({ title: "functions.InlineTasksProfile" });
var AgentInlineAgentBaseWithFallbacksOrRemoteSchema = z.union([AgentInlineAgentBaseWithFallbacksSchema.meta({ "title": "agent.InlineAgentBaseWithFallbacks", "variantTitle": "AgentBase" }), RemotePathSchema.meta({ "title": "RemotePath", "variantTitle": "Remote" })]).describe("An agent specification that is either an inline agent base with fallbacks\nor a remote path reference.\n\nUsed in swarm definitions to allow agents to be specified inline\n(with optional fallbacks) or resolved from a remote source via a\nhashmap during conversion.").meta({ title: "agent.InlineAgentBaseWithFallbacksOrRemote" });

// src/agent/inlineAgentBaseWithFallbacksOrRemoteWithCount.ts
var AgentInlineAgentBaseWithFallbacksOrRemoteWithCountSchema = AgentInlineAgentBaseWithFallbacksOrRemoteSchema.and(z.object({
  count: z.number().int().min(0).max(18446744073709552e3).default(1).describe("Number of instances of this agent in the swarm. Defaults to 1.")
})).describe("An [`InlineAgentBaseWithFallbacksOrRemote`] with a count\n(pre-validation swarm agent slot).").meta({ title: "agent.InlineAgentBaseWithFallbacksOrRemoteWithCount" });

// src/swarm/inlineSwarmBase.ts
var SwarmInlineSwarmBaseSchema = z.object({
  agents: z.array(AgentInlineAgentBaseWithFallbacksOrRemoteWithCountSchema).describe("The LLMs in this swarm, with optional counts and fallbacks."),
  weights: WeightsSchema.nullable().describe("Optional weights for each agent. If `None`, uniform weights are used.").meta({ omitempty: true }).optional()
}).describe("An inline swarm base definition (without computed ID or metadata).\n\nContains a list of agent configurations that will be validated, deduplicated,\nand sorted when converting to an [`InlineSwarm`].").meta({ title: "swarm.InlineSwarmBase" });

// src/functions/inlineProfile.ts
var FunctionsInlineProfileSchema = z.union([z.lazy(() => FunctionsInlineTasksProfileSchema).describe("Tasks-based profile with per-task configuration.").meta({ "title": "functions.InlineTasksProfile", "variantTitle": "Tasks" }), SwarmInlineSwarmBaseSchema.describe("Auto profile that applies a single swarm+weights to all vector completion tasks.").meta({ "title": "swarm.InlineSwarmBase", "variantTitle": "Auto" })]).describe("An inline profile, either tasks-based or auto.").meta({ title: "functions.InlineProfile" });

// src/functions/inlineProfileOrRemoteCommitOptional.ts
var FunctionsInlineProfileOrRemoteCommitOptionalSchema = z.union([FunctionsInlineProfileSchema.meta({ "title": "functions.InlineProfile", "variantTitle": "Inline" }), RemotePathCommitOptionalSchema.meta({ "title": "RemotePathCommitOptional", "variantTitle": "Remote" })]).describe("A profile specification that is either an inline profile definition\nor a remote path reference.").meta({ title: "functions.InlineProfileOrRemoteCommitOptional" });

// src/functions/executions/request/functionExecutionCreateParams.ts
var FunctionsExecutionsRequestFunctionExecutionCreateParamsSchema = z.object({
  continuation: z.string().nullable().describe("Continuation from a previous completion, as a base64-encoded string.").meta({ omitempty: true }).optional(),
  function: FunctionsFullInlineFunctionOrRemoteCommitOptionalSchema.describe("The function to execute (inline definition or remote path)."),
  input: FunctionsExpressionInputValueSchema,
  invert: z.boolean().nullable().describe('If `true`, invert every output in the streamed response *after* the\ninner function has finished computing \u2014 scalar outputs become\n`1 - x`, vector outputs are reversed in place. The expression\nevaluator inside the function still sees the original scores; only\nthe chunks delivered to the client (and the aggregated response\npassed to the usage handler) are inverted. Useful when a function\nis naturally written to score "lower is better" but the consumer\nwants "higher is better", or vice versa.').meta({ omitempty: true }).optional(),
  profile: FunctionsInlineProfileOrRemoteCommitOptionalSchema.describe("The profile to use (inline definition or remote path)."),
  provider: AgentCompletionsRequestProviderSchema.nullable().meta({ omitempty: true }).optional(),
  reasoning: FunctionsExecutionsRequestReasoningSchema.nullable().meta({ omitempty: true }).optional(),
  seed: z.number().int().min(-9223372036854776e3).max(9223372036854776e3).nullable().meta({ omitempty: true }).optional(),
  split: z.boolean().nullable().meta({ omitempty: true }).optional(),
  strategy: FunctionsExecutionsRequestStrategySchema.nullable().meta({ omitempty: true }).optional(),
  stream: z.boolean().nullable().meta({ omitempty: true }).optional()
}).describe("Parameters for creating a function execution.").meta({ title: "functions.executions.request.FunctionExecutionCreateParams" });
var SwarmInlineSwarmBaseOrRemoteCommitOptionalSchema = z.union([SwarmInlineSwarmBaseSchema.meta({ "title": "swarm.InlineSwarmBase", "variantTitle": "SwarmBase" }), RemotePathCommitOptionalSchema.meta({ "title": "RemotePathCommitOptional", "variantTitle": "Remote" })]).describe("Like [`InlineSwarmBaseOrRemote`] but with optional commit.\nUsed in request types where commit resolution happens server-side.").meta({ title: "swarm.InlineSwarmBaseOrRemoteCommitOptional" });

// src/vector/completions/request/vectorCompletionCreateParams.ts
var VectorCompletionsRequestVectorCompletionCreateParamsSchema = z.object({
  continuation: z.string().nullable().describe("Continuation from a previous completion, as a base64-encoded string.").meta({ omitempty: true }).optional(),
  messages: z.array(AgentCompletionsMessageMessageSchema).describe("The conversation messages (the prompt)."),
  provider: AgentCompletionsRequestProviderSchema.nullable().describe("Provider routing preferences.").meta({ omitempty: true }).optional(),
  responses: z.array(AgentCompletionsMessageRichContentSchema).describe("The possible responses the LLMs can vote for."),
  seed: z.number().int().min(-9223372036854776e3).max(9223372036854776e3).nullable().describe("Random seed for deterministic results.").meta({ omitempty: true }).optional(),
  stream: z.boolean().nullable().describe("Whether to stream the response.").meta({ omitempty: true }).optional(),
  swarm: SwarmInlineSwarmBaseOrRemoteCommitOptionalSchema.describe("The Swarm of agents to use.")
}).describe("Parameters for creating a vector completion.\n\nVector completions run multiple agent completions (one per LLM in the\nswarm), force each to vote for one of the predefined responses, and\ncombine votes using the provided profile weights to produce final scores.").meta({ title: "vector.completions.request.VectorCompletionCreateParams" });

// src/cli/command/agents/logs/read/id/response.ts
var CliCommandAgentsLogsReadIdResponseSchema = z.union([z.object({
  body: AgentCompletionsRequestAgentCompletionCreateParamsSchema,
  created_at: z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  response_id: z.string(),
  sender_agent_instance_hierarchy: z.string(),
  type: z.literal("agent_completion_request")
}).meta({ "variantTitle": "AgentCompletionRequest" }), z.object({
  body: VectorCompletionsRequestVectorCompletionCreateParamsSchema,
  created_at: z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  response_id: z.string(),
  sender_agent_instance_hierarchy: z.string(),
  type: z.literal("vector_completion_request")
}).meta({ "variantTitle": "VectorCompletionRequest" }), z.object({
  body: FunctionsExecutionsRequestFunctionExecutionCreateParamsSchema,
  created_at: z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  response_id: z.string(),
  sender_agent_instance_hierarchy: z.string(),
  type: z.literal("function_execution_request")
}).meta({ "variantTitle": "FunctionExecutionRequest" }), z.object({
  text: z.string(),
  type: z.literal("text")
}).meta({ "variantTitle": "Text" }), AgentCompletionsMessageImageUrlSchema.and(z.object({
  type: z.literal("image")
})).meta({ "variantTitle": "Image" }), AgentCompletionsMessageInputAudioSchema.and(z.object({
  type: z.literal("audio")
})).meta({ "variantTitle": "Audio" }), AgentCompletionsMessageVideoUrlSchema.and(z.object({
  type: z.literal("video")
})).meta({ "variantTitle": "Video" }), AgentCompletionsMessageFileSchema.and(z.object({
  type: z.literal("file")
})).meta({ "variantTitle": "File" })]).describe('Resolved payload for one `logs.messages."index"`. Tagged by\n`type`, snake_case discriminant. The MCP projection in\n[`CommandResponse::into_mcp`] hands media variants over as\n[`ContentBlock`]s and text as a bare JSON string \u2014 matching the\nexisting `agents queue read id` projection of `RichContentPart`.\nThe three request-blob variants render as JSONL with their full\ntyped `\u2026CreateParams` body so callers can introspect request\nmetadata.\n\n[`ContentBlock`]: crate::mcp::tool::ContentBlock').meta({ title: "cli.command.agents.logs.read.id.Response" });

// src/viewer/command/agents/logs/read/id.ts
async function agentsLogsReadIdExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/logs/read/id" }), z.union([CliErrorSchema, CliCommandAgentsLogsReadIdResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read id: cli produced no output before the end marker");
  }
  return first;
}
async function agentsLogsReadIdExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/logs/read/id" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read id: cli produced no output before the end marker");
  }
  return first;
}
async function agentsLogsReadIdRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/logs/read/id/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read id request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsLogsReadIdRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/logs/read/id/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read id request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsLogsReadIdResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/logs/read/id/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read id response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsLogsReadIdResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/logs/read/id/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read id response_schema: cli produced no output before the end marker");
  }
  return first;
}
function agentsLogsReadPendingExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/logs/read/pending" }), z.union([CliErrorSchema, CliCommandAgentsLogsReadAllResponseItemSchema]));
}
function agentsLogsReadPendingExecuteTransform(request, transform) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/logs/read/pending" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function agentsLogsReadPendingRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/logs/read/pending/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read pending request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsLogsReadPendingRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/logs/read/pending/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read pending request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsLogsReadPendingResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/logs/read/pending/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read pending response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsLogsReadPendingResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/logs/read/pending/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read pending response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandAgentsLogsReadSubscribeAgentsInactiveTagSchema = z.literal("agents_inactive").describe('Single-variant enum whose lone variant serializes as the\nliteral string `"agents_inactive"`. Construct as\n`AgentsInactiveTag::AgentsInactive`; the surrounding\n`ResponseItem::AgentsInactive(_)` carries it.').meta({ title: "cli.command.agents.logs.read.subscribe.AgentsInactiveTag" });

// src/cli/command/agents/logs/read/subscribe/responseItem.ts
var CliCommandAgentsLogsReadSubscribeResponseItemSchema = z.union([CliCommandAgentsLogsReadAllResponseItemSchema.meta({ "title": "cli.command.agents.logs.read.all.ResponseItem", "variantTitle": "Item" }), CliCommandAgentsLogsReadSubscribeAgentsInactiveTagSchema.meta({ "title": "cli.command.agents.logs.read.subscribe.AgentsInactiveTag", "variantTitle": "AgentsInactive" })]).describe('Subscribe\'s wire shape. Either a real parts-grouped block\n(the EXACT same enum `read all` / `read pending` emit) OR\nthe literal string `"agents_inactive"`. `#[serde(untagged)]`\nso the `Item` arm passes through transparently \u2014 JSONL\nconsumers see either a block JSON object or a bare string.').meta({ title: "cli.command.agents.logs.read.subscribe.ResponseItem" });

// src/viewer/command/agents/logs/read/subscribe.ts
function agentsLogsReadSubscribeExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/logs/read/subscribe" }), z.union([CliErrorSchema, CliCommandAgentsLogsReadSubscribeResponseItemSchema]));
}
function agentsLogsReadSubscribeExecuteTransform(request, transform) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/logs/read/subscribe" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function agentsLogsReadSubscribeRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/logs/read/subscribe/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsLogsReadSubscribeRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/logs/read/subscribe/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsLogsReadSubscribeResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/logs/read/subscribe/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsLogsReadSubscribeResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/logs/read/subscribe/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents logs read subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandAgentsMessageResponseSchema = z.union([z.object({
  type: z.literal("delivered")
}).describe("The queue row reached a live agent (its row flipped to\ninactive \u2014 the API stamped its id onto an assistant chunk's\n`request_message_ids`) before the agent's lock freed up.").meta({ "variantTitle": "Delivered" }), z.object({
  agent_instance_hierarchy: z.string(),
  type: z.literal("id")
}).describe("The handler execed a detached `agents spawn` child (with the\nagent's lock transferred into it) and the child yielded its\n`Id` first item \u2014 the bare `agent_instance_hierarchy` the\nrunner just minted or resumed.").meta({ "variantTitle": "Id" })]).describe('Unary response. Exactly one of these per call. Internally tagged\nvia `type`; bare unit variant `Delivered` serializes as\n`{"type":"delivered"}`.').meta({ title: "cli.command.agents.message.Response" });

// src/viewer/command/agents/message.ts
async function agentsMessageExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/message" }), z.union([CliErrorSchema, CliCommandAgentsMessageResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents message: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMessageExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/message" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents message: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMessageRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/message/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents message request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMessageRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/message/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents message request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMessageResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/message/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents message response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMessageResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/message/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents message response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandAgentsPublishResponseSchema = z.object({
  sha: z.string()
}).meta({ title: "cli.command.agents.publish.Response" });

// src/viewer/command/agents/publish.ts
async function agentsPublishExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/publish" }), z.union([CliErrorSchema, CliCommandAgentsPublishResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents publish: cli produced no output before the end marker");
  }
  return first;
}
async function agentsPublishExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/publish" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents publish: cli produced no output before the end marker");
  }
  return first;
}
async function agentsPublishRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/publish/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents publish request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsPublishRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/publish/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents publish request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsPublishResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/publish/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents publish response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsPublishResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/publish/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents publish response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandAgentsQueueDeleteResponseSchema = z.object({
  agent_instance_hierarchy: z.string().nullable().meta({ omitempty: true }).optional(),
  agent_tag: z.string().nullable().meta({ omitempty: true }).optional(),
  content: AgentCompletionsMessageRichContentSchema,
  enqueued_at: z.string().describe("RFC3339 timestamp the dropped row was enqueued at."),
  id: z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  key: z.string().nullable().describe("Idempotency token, if the dropped row had one.").meta({ omitempty: true }).optional()
}).describe("What was deleted. Carries every column of the original\n`prompts` row so the caller can confirm the drop:\nexactly one of `agent_instance_hierarchy` / `agent_tag` is set\n(matching the original target), `enqueued_at` is the original\nunix-seconds timestamp, and `content` is the reconstructed\n`RichContent` body.").meta({ title: "cli.command.agents.queue.delete.Response" });

// src/viewer/command/agents/queue/delete.ts
async function agentsQueueDeleteExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/queue/delete" }), z.union([CliErrorSchema, CliCommandAgentsQueueDeleteResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue delete: cli produced no output before the end marker");
  }
  return first;
}
async function agentsQueueDeleteExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/queue/delete" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue delete: cli produced no output before the end marker");
  }
  return first;
}
async function agentsQueueDeleteRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/queue/delete/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue delete request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsQueueDeleteRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/queue/delete/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue delete request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsQueueDeleteResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/queue/delete/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue delete response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsQueueDeleteResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/queue/delete/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue delete response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandAgentsQueueDeliverAgentActiveTypeSchema = z.literal("AgentActive").meta({ title: "cli.command.agents.queue.deliver.AgentActiveType" });

// src/cli/command/agents/queue/deliver/agentActiveResponseItem.ts
var CliCommandAgentsQueueDeliverAgentActiveResponseItemSchema = z.object({
  agent_instance_hierarchy: z.string(),
  type: CliCommandAgentsQueueDeliverAgentActiveTypeSchema
}).describe("This agent's lock was held by a live owner \u2014 it is already active\nand will drain its own queue; nothing was spawned for it.").meta({ title: "cli.command.agents.queue.deliver.AgentActiveResponseItem" });
var CliCommandAgentsQueueDeliverAgentSpawnedTypeSchema = z.literal("AgentSpawned").meta({ title: "cli.command.agents.queue.deliver.AgentSpawnedType" });

// src/cli/command/agents/queue/deliver/agentSpawnedResponseItem.ts
var CliCommandAgentsQueueDeliverAgentSpawnedResponseItemSchema = z.object({
  agent_instance_hierarchy: z.string(),
  type: CliCommandAgentsQueueDeliverAgentSpawnedTypeSchema
}).describe("This agent's lock was won and its spawn has started; its output\nfollows as [`ValueResponseItem`]s (in `stream_spawns` mode).").meta({ title: "cli.command.agents.queue.deliver.AgentSpawnedResponseItem" });
var CliCommandAgentsQueueDeliverAllAgentsActiveSchema = z.literal("AllAgentsActive").describe('Every target has resolved to active-or-spawned. Wire shape is the\nbare string `"AllAgentsActive"` (a one-variant enum \u2014 a unit\nvariant in the untagged [`ResponseItem`] would serialize as\n`null`, not the marker string). The detached default mode stops\nreading its child at this item.').meta({ title: "cli.command.agents.queue.deliver.AllAgentsActive" });
var CliCommandAgentsQueueDeliverTagActiveTypeSchema = z.literal("TagActive").meta({ title: "cli.command.agents.queue.deliver.TagActiveType" });

// src/cli/command/agents/queue/deliver/tagActiveResponseItem.ts
var CliCommandAgentsQueueDeliverTagActiveResponseItemSchema = z.object({
  agent_tag: z.string(),
  type: CliCommandAgentsQueueDeliverTagActiveTypeSchema
}).describe("This un-upgraded tag's lock was held by a live owner \u2014 another\nprocess is already materializing it; the queued rows will reach\nthe agent it mints. Nothing was spawned for it.").meta({ title: "cli.command.agents.queue.deliver.TagActiveResponseItem" });
var CliCommandAgentsQueueDeliverTagSpawnedTypeSchema = z.literal("TagSpawned").meta({ title: "cli.command.agents.queue.deliver.TagSpawnedType" });

// src/cli/command/agents/queue/deliver/tagSpawnedResponseItem.ts
var CliCommandAgentsQueueDeliverTagSpawnedResponseItemSchema = z.object({
  agent_tag: z.string(),
  type: CliCommandAgentsQueueDeliverTagSpawnedTypeSchema
}).describe("This un-upgraded tag's lock was won and a fresh spawn of the\ngroup's stored agent spec has started. The minted\n`agent_instance_hierarchy` isn't known yet at this point \u2014 it\narrives as the FIRST inner item (the spawn `Id`) of the\n[`ValueResponseItem`]s that follow (in `stream_spawns` mode).").meta({ title: "cli.command.agents.queue.deliver.TagSpawnedResponseItem" });
var CliCommandAgentsQueueDeliverValueResponseItemSchema = z.object({
  agent_instance_hierarchy: z.string().describe("The delivered agent's `agent_instance_hierarchy`."),
  value: JsonValueSchema.describe("The typed root item the spawn emitted.")
}).describe("One output item from one delivered agent's spawn stream. `value`\nis the typed root [`crate::cli::command::ResponseItem`] (the spawn\nitem wrapped at the root) \u2014 boxed because the root union\ntransitively contains *this* type (`agents \u2192 queue \u2192 deliver`),\nand boxing is what makes the recursion sized.\n\nThe `value` field's JSON schema is opaqued to `serde_json::Value`\n(renders as bare `{}` aka JsonValue) so the published schema\ndoesn't inline the entire root union \u2014 that's the TS7056 blowup\nthe root and tier aggregates dodge by being `json_schema_ignore`.\nDownstream SDKs see `value: JsonValue` on the typed `execute`\npath; consumers that want to peer inside parse it case-by-case.").meta({ title: "cli.command.agents.queue.deliver.ValueResponseItem" });

// src/cli/command/agents/queue/deliver/responseItem.ts
var CliCommandAgentsQueueDeliverResponseItemSchema = z.union([CliCommandAgentsQueueDeliverValueResponseItemSchema.meta({ "title": "cli.command.agents.queue.deliver.ValueResponseItem", "variantTitle": "Value" }), CliCommandAgentsQueueDeliverAgentActiveResponseItemSchema.meta({ "title": "cli.command.agents.queue.deliver.AgentActiveResponseItem", "variantTitle": "AgentActive" }), CliCommandAgentsQueueDeliverAgentSpawnedResponseItemSchema.meta({ "title": "cli.command.agents.queue.deliver.AgentSpawnedResponseItem", "variantTitle": "AgentSpawned" }), CliCommandAgentsQueueDeliverTagActiveResponseItemSchema.meta({ "title": "cli.command.agents.queue.deliver.TagActiveResponseItem", "variantTitle": "TagActive" }), CliCommandAgentsQueueDeliverTagSpawnedResponseItemSchema.meta({ "title": "cli.command.agents.queue.deliver.TagSpawnedResponseItem", "variantTitle": "TagSpawned" }), CliCommandAgentsQueueDeliverAllAgentsActiveSchema.meta({ "title": "cli.command.agents.queue.deliver.AllAgentsActive", "variantTitle": "AllAgentsActive" })]).describe('One stream item from `agents queue deliver`. Untagged \u2014 the\nvariants are disjoint on the wire: `Value` requires `value`,\n`AgentActive` / `AgentSpawned` / `TagActive` / `TagSpawned` carry\ndistinct `type` markers, and `AllAgentsActive` is the bare string\n`"AllAgentsActive"`.').meta({ title: "cli.command.agents.queue.deliver.ResponseItem" });

// src/viewer/command/agents/queue/deliver.ts
function agentsQueueDeliverExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/queue/deliver" }), z.union([CliErrorSchema, CliCommandAgentsQueueDeliverResponseItemSchema]));
}
function agentsQueueDeliverExecuteTransform(request, transform) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/queue/deliver" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function agentsQueueDeliverRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/queue/deliver/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue deliver request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsQueueDeliverRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/queue/deliver/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue deliver request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsQueueDeliverResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/queue/deliver/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue deliver response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsQueueDeliverResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/queue/deliver/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue deliver response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsQueueReadIdExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/queue/read/id" }), z.union([CliErrorSchema, AgentCompletionsMessageRichContentPartSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue read id: cli produced no output before the end marker");
  }
  return first;
}
async function agentsQueueReadIdExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/queue/read/id" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue read id: cli produced no output before the end marker");
  }
  return first;
}
async function agentsQueueReadIdRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/queue/read/id/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue read id request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsQueueReadIdRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/queue/read/id/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue read id request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsQueueReadIdResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/queue/read/id/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue read id response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsQueueReadIdResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/queue/read/id/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue read id response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandAgentsQueueReadPendingQueuePartSchema = z.object({
  id: z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  type: CliCommandAgentsLogsReadAllClientNotificationPartTypeSchema
}).describe("One row inside a `ResponseItem` block \u2014 a\n`message_queue_contents` entry. The `id` is the\n`message_queue_contents.id`, which you pass to\n`agents queue read id <n>` to drill into the body.\n`enqueued_at` is on the enclosing block, not here\n(one block = one `message_queue` parent row, sharing one\n`enqueued_at`).").meta({ title: "cli.command.agents.queue.read.pending.QueuePart" });

// src/cli/command/agents/queue/read/pending/responseItem.ts
var CliCommandAgentsQueueReadPendingResponseItemSchema = z.union([z.object({
  agent_instance_hierarchy: z.string(),
  by: z.literal("agent_instance_hierarchy"),
  delete_id: z.number().int().min(-9223372036854776e3).max(9223372036854776e3).describe("`message_queue.id` \u2014 the row-level id this block\nrepresents. Pass to `agents queue delete <id>` to\nsoft-flip the entire row (all parts) in one call.\nDistinct from each `QueuePart.id` (which is a\n`message_queue_contents.id` for drilling into one\ncontent slot via `agents queue read id`)."),
  enqueued_at: z.string().describe("`message_queue.enqueued_at`. One block = one parent\n`message_queue` row, so this is well-defined\nblock-level."),
  key: z.string().nullable().describe("Idempotency token, if the row was enqueued with `--key`.").meta({ omitempty: true }).optional(),
  parts: z.array(CliCommandAgentsQueueReadPendingQueuePartSchema),
  sender_agent_instance_hierarchy: z.string().describe("AIH of the caller who enqueued \u2014 from\n`message_queue.sender_*`.")
}).meta({ "variantTitle": "AgentInstanceHierarchy" }), z.object({
  agent_tag: z.string(),
  by: z.literal("tag"),
  delete_id: z.number().int().min(-9223372036854776e3).max(9223372036854776e3).describe("`message_queue.id`. Pass to `agents queue delete <id>`."),
  enqueued_at: z.string(),
  key: z.string().nullable().meta({ omitempty: true }).optional(),
  parts: z.array(CliCommandAgentsQueueReadPendingQueuePartSchema),
  sender_agent_instance_hierarchy: z.string()
}).meta({ "variantTitle": "Tag" })]).describe("One pending `message_queue` row, with its content rows\ngrouped as `parts`. Two variants \u2014 direct AIH target or tag\ntarget \u2014 both flat (no nested `LookupState`).").meta({ title: "cli.command.agents.queue.read.pending.ResponseItem" });

// src/viewer/command/agents/queue/read/pending.ts
function agentsQueueReadPendingExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/queue/read/pending" }), z.union([CliErrorSchema, CliCommandAgentsQueueReadPendingResponseItemSchema]));
}
function agentsQueueReadPendingExecuteTransform(request, transform) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/queue/read/pending" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function agentsQueueReadPendingRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/queue/read/pending/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue read pending request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsQueueReadPendingRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/queue/read/pending/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue read pending request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsQueueReadPendingResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/queue/read/pending/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue read pending response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsQueueReadPendingResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/queue/read/pending/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents queue read pending response_schema: cli produced no output before the end marker");
  }
  return first;
}
var AgentCompletionsMessageAssistantToolCallFunctionDeltaSchema = z.object({
  arguments: z.string().nullable().describe("The arguments being streamed (accumulated across deltas).").meta({ omitempty: true }).optional(),
  name: z.string().nullable().describe("The function name (only present in the first delta).").meta({ omitempty: true }).optional()
}).describe("Function call details in a streaming tool call.").meta({ title: "agent.completions.message.AssistantToolCallFunctionDelta" });
var AgentCompletionsMessageAssistantToolCallTypeSchema = z.literal("function").describe("A function call.").meta({ title: "agent.completions.message.AssistantToolCallType" });

// src/agent/completions/message/assistantToolCallDelta.ts
var AgentCompletionsMessageAssistantToolCallDeltaSchema = z.object({
  function: AgentCompletionsMessageAssistantToolCallFunctionDeltaSchema.nullable().describe("The function call details.").meta({ omitempty: true }).optional(),
  id: z.string().nullable().describe("The unique ID of this tool call.").meta({ omitempty: true }).optional(),
  index: z.number().int().min(0).max(18446744073709552e3).describe("The index of this tool call."),
  type: AgentCompletionsMessageAssistantToolCallTypeSchema.nullable().describe('The type of tool call (always "function").').meta({ omitempty: true }).optional()
}).describe("A tool call delta in a streaming response.").meta({ title: "agent.completions.message.AssistantToolCallDelta" });
var AgentCompletionsResponseAssistantRoleSchema = z.literal("assistant").describe("The assistant role.").meta({ title: "agent.completions.response.AssistantRole" });
var AgentCompletionsResponseFinishReasonSchema = z.union([z.literal("stop").describe("The model reached a natural stop point or stop sequence.").meta({ "variantTitle": "Stop" }), z.literal("length").describe("The model reached the maximum token limit.").meta({ "variantTitle": "Length" }), z.literal("tool_calls").describe("The model decided to call one or more tools.").meta({ "variantTitle": "ToolCalls" }), z.literal("content_filter").describe("The response was filtered due to content policy.").meta({ "variantTitle": "ContentFilter" }), z.literal("error").describe("An error occurred during generation.").meta({ "variantTitle": "Error" })]).describe("The reason the model stopped generating.").meta({ title: "agent.completions.response.FinishReason" });
var AgentCompletionsResponseTopLogprobSchema = z.object({
  bytes: z.array(z.number().int().min(0).max(255)).nullable().describe("The raw bytes of the token.").optional(),
  logprob: z.number().min(-34028234663852886e22).max(34028234663852886e22).nullable().describe("The log probability of this token.").optional(),
  token: z.string().describe("The token string.")
}).describe("A top alternative token with its log probability.").meta({ title: "agent.completions.response.TopLogprob" });

// src/agent/completions/response/logprob.ts
var AgentCompletionsResponseLogprobSchema = z.object({
  bytes: z.array(z.number().int().min(0).max(255)).nullable().describe("The raw bytes of the token.").optional(),
  logprob: z.number().min(-34028234663852886e22).max(34028234663852886e22).describe("The log probability of this token."),
  token: z.string().describe("The token string."),
  top_logprobs: z.array(AgentCompletionsResponseTopLogprobSchema).describe("The top alternative tokens and their log probabilities.")
}).describe("Log probability information for a single token.").meta({ title: "agent.completions.response.Logprob" });

// src/agent/completions/response/logprobs.ts
var AgentCompletionsResponseLogprobsSchema = z.object({
  content: z.array(AgentCompletionsResponseLogprobSchema).nullable().describe("Log probabilities for content tokens.").optional(),
  refusal: z.array(AgentCompletionsResponseLogprobSchema).nullable().describe("Log probabilities for refusal tokens.").optional()
}).describe("Log probabilities for generated tokens.").meta({ title: "agent.completions.response.Logprobs" });
var AgentCompletionsResponseCompletionTokensDetailsSchema = z.object({
  accepted_prediction_tokens: z.number().int().min(0).max(18446744073709552e3).nullable().describe("Tokens from accepted predictions (speculative decoding).").meta({ omitempty: true }).optional(),
  audio_tokens: z.number().int().min(0).max(18446744073709552e3).nullable().describe("Audio output tokens.").meta({ omitempty: true }).optional(),
  reasoning_tokens: z.number().int().min(0).max(18446744073709552e3).nullable().describe("Tokens used for reasoning/thinking.").meta({ omitempty: true }).optional(),
  rejected_prediction_tokens: z.number().int().min(0).max(18446744073709552e3).nullable().describe("Tokens from rejected predictions (speculative decoding).").meta({ omitempty: true }).optional()
}).describe("Detailed breakdown of completion token usage.").meta({ title: "agent.completions.response.CompletionTokensDetails" });
var AgentCompletionsResponseCostDetailsSchema = z.object({
  upstream_inference_cost: z.number().min(-34028234663852886e22).max(34028234663852886e22).describe("Cost charged by the immediate upstream (e.g., OpenRouter)."),
  upstream_upstream_inference_cost: z.number().min(-34028234663852886e22).max(34028234663852886e22).describe("Cost charged by the upstream's upstream (e.g., the actual model provider).")
}).describe("Detailed cost breakdown.").meta({ title: "agent.completions.response.CostDetails" });
var AgentCompletionsResponsePromptTokensDetailsSchema = z.object({
  audio_tokens: z.number().int().min(0).max(18446744073709552e3).nullable().describe("Audio input tokens.").meta({ omitempty: true }).optional(),
  cache_write_tokens: z.number().int().min(0).max(18446744073709552e3).nullable().describe("Tokens written to cache.").meta({ omitempty: true }).optional(),
  cached_tokens: z.number().int().min(0).max(18446744073709552e3).nullable().describe("Tokens served from cache.").meta({ omitempty: true }).optional(),
  video_tokens: z.number().int().min(0).max(18446744073709552e3).nullable().describe("Video input tokens.").meta({ omitempty: true }).optional()
}).describe("Detailed breakdown of prompt token usage.").meta({ title: "agent.completions.response.PromptTokensDetails" });

// src/agent/completions/response/upstreamUsage.ts
var AgentCompletionsResponseUpstreamUsageSchema = z.object({
  completion_tokens: z.number().int().min(0).max(18446744073709552e3).describe("Number of tokens in the completion."),
  completion_tokens_details: AgentCompletionsResponseCompletionTokensDetailsSchema.nullable().describe("Detailed breakdown of completion tokens.").meta({ omitempty: true }).optional(),
  cost: z.number().min(-34028234663852886e22).max(34028234663852886e22).describe("The cost charged by ObjectiveAI for this request."),
  cost_details: AgentCompletionsResponseCostDetailsSchema.nullable().describe("Detailed cost breakdown.").meta({ omitempty: true }).optional(),
  cost_multiplier: z.number().min(-34028234663852886e22).max(34028234663852886e22).describe("The multiplier applied to compute ObjectiveAI's charge."),
  is_byok: z.boolean().describe("Whether this request used Bring Your Own Key (BYOK)."),
  prompt_tokens: z.number().int().min(0).max(18446744073709552e3).describe("Number of tokens in the prompt."),
  prompt_tokens_details: AgentCompletionsResponsePromptTokensDetailsSchema.nullable().describe("Detailed breakdown of prompt tokens.").meta({ omitempty: true }).optional(),
  total_cost: z.number().min(-34028234663852886e22).max(34028234663852886e22).describe("Total cost including ObjectiveAI's charge plus all upstream charges.\nFor BYOK requests, ObjectiveAI only charges the cost_multiplier difference,\nbut total_cost still includes what the upstream provider charged."),
  total_tokens: z.number().int().min(0).max(18446744073709552e3).describe("Total tokens (prompt + completion).")
}).describe("Token usage and cost information from an upstream provider.\n\nThis is the per-assistant-response usage yielded by upstream clients.\nIt includes upstream-specific fields like `cost_multiplier` and `is_byok`.").meta({ title: "agent.completions.response.UpstreamUsage" });

// src/agent/completions/response/streaming/assistantResponseChunk.ts
var AgentCompletionsResponseStreamingAssistantResponseChunkSchema = z.object({
  content: AgentCompletionsMessageRichContentSchema.nullable().meta({ omitempty: true }).optional(),
  created: z.number().int().min(0).max(18446744073709552e3),
  finish_reason: AgentCompletionsResponseFinishReasonSchema.nullable().optional(),
  index: z.number().int().min(0).max(18446744073709552e3),
  logprobs: AgentCompletionsResponseLogprobsSchema.nullable().meta({ omitempty: true }).optional(),
  model: z.string(),
  provider: z.string().nullable().meta({ omitempty: true }).optional(),
  reasoning: z.string().nullable().meta({ omitempty: true }).optional(),
  refusal: z.string().nullable().meta({ omitempty: true }).optional(),
  request_message_ids: z.array(z.number().int().min(-9223372036854776e3).max(9223372036854776e3)).nullable().describe("`message_queue_contents.id`s the API consumed to seed this\nturn's request. Stamped onto the first assistant chunk the\nAPI emits downstream \u2014 when set, the consumer owns these\nrows. Kinds are resolved CLI-side at write time (SQL CASE\nagainst `message_queue_contents.kind`), so the wire stays\n`i64`-only. Both `None` and `Some(empty)` are skipped on\nserialize.").meta({ omitempty: true }).optional(),
  role: AgentCompletionsResponseAssistantRoleSchema,
  service_tier: z.string().nullable().meta({ omitempty: true }).optional(),
  system_fingerprint: z.string().nullable().meta({ omitempty: true }).optional(),
  tool_calls: z.array(AgentCompletionsMessageAssistantToolCallDeltaSchema).nullable().meta({ omitempty: true }).optional(),
  upstream_id: z.string(),
  usage: AgentCompletionsResponseUpstreamUsageSchema.nullable().describe("Upstream usage for this assistant response (set by upstream clients).").meta({ omitempty: true }).optional()
}).describe("A chunk of a streaming agent completion response.\n\nMultiple chunks are received via Server-Sent Events and can be\naccumulated into a complete [`AgentCompletion`](response::unary::AgentCompletion)\nusing the [`push`](Self::push) method.").meta({ title: "agent.completions.response.streaming.AssistantResponseChunk" });
var AgentCompletionsResponseToolRoleSchema = z.literal("tool").meta({ title: "agent.completions.response.ToolRole" });

// src/agent/completions/response/toolResponse.ts
var AgentCompletionsResponseToolResponseSchema = z.object({
  content: AgentCompletionsMessageRichContentSchema.describe("The content of the tool response."),
  index: z.number().int().min(0).max(18446744073709552e3),
  metadata: AgentCompletionsMessageToolResponseMetadataSchema.nullable().describe("Optional vendor-extension metadata, populated by\n`objectiveai-mcp-proxy` via MCP's `_meta` extension bag.").meta({ omitempty: true }).optional(),
  request_message_ids: z.array(z.number().int().min(-9223372036854776e3).max(9223372036854776e3)).nullable().describe("Mirrors `AssistantResponseChunk.request_message_ids` \u2014\nthe consumed `message_queue_contents.id`s. Currently never\npopulated (the API stamps the assistant chunk instead);\nthe field exists so the wire shape is symmetric across\nthe two `MessageChunk` variants. Both `None` and\n`Some(empty)` are skipped on serialize.").meta({ omitempty: true }).optional(),
  role: AgentCompletionsResponseToolRoleSchema,
  tool_call_id: z.string().describe("The ID of the tool call this message responds to.")
}).describe("A tool message containing the result of a tool call.").meta({ title: "agent.completions.response.ToolResponse" });

// src/agent/completions/response/streaming/messageChunk.ts
var AgentCompletionsResponseStreamingMessageChunkSchema = z.union([AgentCompletionsResponseStreamingAssistantResponseChunkSchema.meta({ "title": "agent.completions.response.streaming.AssistantResponseChunk", "variantTitle": "Assistant" }), AgentCompletionsResponseToolResponseSchema.meta({ "title": "agent.completions.response.ToolResponse", "variantTitle": "Tool" })]).meta({ title: "agent.completions.response.streaming.MessageChunk" });
var AgentCompletionsResponseStreamingObjectSchema = z.literal("agent.completion.chunk").describe("A agent completion chunk object.").meta({ title: "agent.completions.response.streaming.Object" });
var AgentCompletionsResponseUsageSchema = z.object({
  completion_tokens: z.number().int().min(0).max(18446744073709552e3).describe("Total tokens generated across all assistant responses."),
  completion_tokens_details: AgentCompletionsResponseCompletionTokensDetailsSchema.nullable().describe("Breakdown of completion tokens (reasoning, audio, etc.) if available.").meta({ omitempty: true }).optional(),
  cost: z.number().min(-34028234663852886e22).max(34028234663852886e22).describe("Cost charged by ObjectiveAI for this request."),
  cost_details: AgentCompletionsResponseCostDetailsSchema.nullable().describe("Breakdown of upstream and upstream_upstream costs if available.").meta({ omitempty: true }).optional(),
  prompt_tokens: z.number().int().min(0).max(18446744073709552e3).describe("Total prompt tokens across all assistant responses."),
  prompt_tokens_details: AgentCompletionsResponsePromptTokensDetailsSchema.nullable().describe("Breakdown of prompt tokens (cached, audio, etc.) if available.").meta({ omitempty: true }).optional(),
  total_cost: z.number().min(-34028234663852886e22).max(34028234663852886e22).describe("Total cost including upstream provider charges. Only differs from `cost`\nwhen using BYOK (Bring Your Own Key)."),
  total_tokens: z.number().int().min(0).max(18446744073709552e3).describe("Sum of completion and prompt tokens.")
}).describe('Aggregated token and cost usage for an agent completion.\n\nThis is the "primary" usage type that aggregates across all upstream\nassistant responses within a single agent completion.').meta({ title: "agent.completions.response.Usage" });
var AgentUpstreamSchema = z.union([z.literal("unknown").describe("Unknown Upstream.").meta({ "variantTitle": "Unknown" }), z.literal("openrouter").describe("OpenRouter Upstream.").meta({ "variantTitle": "Openrouter" }), z.literal("claude_agent_sdk").describe("Claude Agent SDK Upstream.").meta({ "variantTitle": "ClaudeAgentSdk" }), z.literal("codex_sdk").describe("Codex SDK Upstream.").meta({ "variantTitle": "CodexSdk" }), z.literal("mock").describe("Mock Upstream.").meta({ "variantTitle": "Mock" })]).describe("Supported agent upstreams.").meta({ title: "agent.Upstream" });
var ErrorResponseErrorSchema = z.object({
  code: z.number().int().min(0).max(65535).describe("The HTTP status code of the error response."),
  message: JsonValueSchema.describe("The error message or details as a JSON value.")
}).describe("An error returned by the ObjectiveAI API.\n\nThis struct represents an API error response containing an HTTP status\ncode and a message. The message can be any JSON value, allowing for\nboth simple string errors and structured error objects.").meta({ title: "error.ResponseError" });

// src/agent/completions/response/streaming/agentCompletionChunk.ts
var AgentCompletionsResponseStreamingAgentCompletionChunkSchema = z.object({
  agent_full_id: z.string().describe("WF-level id: concatenation of the primary agent's id with all\nfallback ids (see `InlineAgentWithFallbacks::full_id`). Same\nfor every slot in the same WF request."),
  agent_id: z.string().describe("Leaf agent id of the slot that produced this chunk. For the\nprimary attempt this is the primary agent's id; on fallback it\nis the fallback agent's id. Same on every chunk of a slot."),
  agent_instance_hierarchy: z.string().describe("Full agent instance hierarchy for this completion's slot \u2014\n`{ctx lineage}/{agent_full_id}-{response_id}`, or the fixed\ncontinuation value on resume. Same on every chunk of a slot."),
  agent_remote: RemotePathSchema.nullable().describe("`RemotePath` the WF was fetched from. `None` when the WF was\nsupplied inline. Same for every slot in the same WF request.").meta({ omitempty: true }).optional(),
  continuation: z.string().nullable().describe("Continuation state for multi-turn conversations (only present in the final chunk).").meta({ omitempty: true }).optional(),
  created: z.number().int().min(0).max(18446744073709552e3),
  error: ErrorResponseErrorSchema.nullable().describe("Error details if this completion failed.").meta({ omitempty: true }).optional(),
  id: z.string(),
  messages: z.array(AgentCompletionsResponseStreamingMessageChunkSchema),
  messages_queued: z.boolean().nullable().describe("`true` when the MCP proxy holds queued messages that were not\ndelivered to the agent via a tool response on this turn. Only\nset when `continuation` is also set \u2014 the caller acts on it by\nissuing the continuation. Absent when nothing is queued, when\nthere is no continuation to act on, or when the peek failed\n(the failure is surfaced via `error`).").meta({ omitempty: true }).optional(),
  object: AgentCompletionsResponseStreamingObjectSchema.describe('The object type (always "agent.completion.chunk").'),
  upstream: AgentUpstreamSchema.describe("Upstream provider"),
  usage: AgentCompletionsResponseUsageSchema.nullable().describe("Token usage (only present in the final chunk).").meta({ omitempty: true }).optional()
}).describe("A chunk of a streaming agent completion response.\n\nMultiple chunks are received via Server-Sent Events and can be\naccumulated into a complete [`AgentCompletion`](response::unary::AgentCompletion)\nusing the [`push`](Self::push) method.").meta({ title: "agent.completions.response.streaming.AgentCompletionChunk" });

// src/cli/command/agents/spawn/responseItem.ts
var CliCommandAgentsSpawnResponseItemSchema = z.union([AgentCompletionsResponseStreamingAgentCompletionChunkSchema.meta({ "title": "agent.completions.response.streaming.AgentCompletionChunk", "variantTitle": "Chunk" }), z.string().meta({ "variantTitle": "Id" })]).meta({ title: "cli.command.agents.spawn.ResponseItem" });

// src/viewer/command/agents/spawn.ts
function agentsSpawnExecuteStreaming(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, dangerous_advanced: { ...request.dangerous_advanced ?? {}, stream: true }, path_type: "agents/spawn" }), z.union([CliErrorSchema, CliCommandAgentsSpawnResponseItemSchema]));
}
function agentsSpawnExecuteStreamingTransform(request, transform) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, dangerous_advanced: { ...request.dangerous_advanced ?? {}, stream: true }, path_type: "agents/spawn" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function agentsSpawnExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, dangerous_advanced: request.dangerous_advanced ? { ...request.dangerous_advanced, stream: void 0 } : request.dangerous_advanced, path_type: "agents/spawn" }), z.union([CliErrorSchema, z.string()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents spawn: cli produced no output before the end marker");
  }
  return first;
}
async function agentsSpawnExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, dangerous_advanced: request.dangerous_advanced ? { ...request.dangerous_advanced, stream: void 0 } : request.dangerous_advanced, path_type: "agents/spawn" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents spawn: cli produced no output before the end marker");
  }
  return first;
}
async function agentsSpawnRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/spawn/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents spawn request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsSpawnRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/spawn/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents spawn request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsSpawnResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/spawn/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents spawn response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsSpawnResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/spawn/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents spawn response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandAgentsTagsApplyResponseSchema = z.union([z.object({
  agent_instance: z.string(),
  agent_instance_hierarchy: z.string().describe("`{parent}/{agent_instance}`."),
  by: z.literal("agent_instance"),
  name: z.string(),
  parent_agent_instance_hierarchy: z.string()
}).meta({ "variantTitle": "AgentInstance" }), z.object({
  agent_spec: AgentInlineAgentBaseWithFallbacksOrRemoteCommitOptionalSchema,
  by: z.literal("agent"),
  name: z.string(),
  parent_agent_instance_hierarchy: z.string(),
  tag_group_id: z.number().int().min(-9223372036854776e3).max(9223372036854776e3)
}).meta({ "variantTitle": "Agent" }), z.union([z.object({
  agent_instance_hierarchy: z.string(),
  agent_tag: z.string(),
  by: z.literal("agent_tag"),
  name: z.string(),
  state: z.literal("bound")
}).meta({ "variantTitle": "Bound" }), z.object({
  agent_spec: AgentInlineAgentBaseWithFallbacksOrRemoteCommitOptionalSchema,
  agent_tag: z.string(),
  by: z.literal("agent_tag"),
  name: z.string(),
  parent_agent_instance_hierarchy: z.string(),
  state: z.literal("grouped"),
  tag_group_id: z.number().int().min(-9223372036854776e3).max(9223372036854776e3)
}).meta({ "variantTitle": "Grouped" })]).describe('Wire shape mirrors the resolved source state:\n`{"by":"agent_tag","name":...,"agent_tag":...,"state":"bound"|"grouped", \u2026}`.').meta({ "variantTitle": "AgentTag" })]).describe("Apply response. Each variant carries the freshly-applied state\nso callers don't need a follow-up lookup.").meta({ title: "cli.command.agents.tags.apply.Response" });

// src/viewer/command/agents/tags/apply.ts
async function agentsTagsApplyExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/tags/apply" }), z.union([CliErrorSchema, CliCommandAgentsTagsApplyResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tags apply: cli produced no output before the end marker");
  }
  return first;
}
async function agentsTagsApplyExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/tags/apply" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tags apply: cli produced no output before the end marker");
  }
  return first;
}
async function agentsTagsApplyRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/tags/apply/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tags apply request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsTagsApplyRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/tags/apply/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tags apply request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsTagsApplyResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/tags/apply/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tags apply response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsTagsApplyResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/tags/apply/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tags apply response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsTagsLookupRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/tags/lookup/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tags lookup request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsTagsLookupRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/tags/lookup/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tags lookup request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsTagsLookupResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/tags/lookup/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tags lookup response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsTagsLookupResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/tags/lookup/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tags lookup response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandOkSchema = z.literal("Ok").describe('Success-only response shape. Wire form is the bare string `"Ok"` \u2014\na single-variant enum gives us a typed sentinel that serializes and\ndeserializes through serde as the static string. Used as `Response`\non every cli leaf whose only success signal is "it worked."').meta({ title: "cli.command.Ok" });

// src/viewer/command/agents/wait.ts
async function agentsWaitExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/wait" }), z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents wait: cli produced no output before the end marker");
  }
  return first;
}
async function agentsWaitExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/wait" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents wait: cli produced no output before the end marker");
  }
  return first;
}
async function agentsWaitRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/wait/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents wait request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsWaitRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/wait/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents wait request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsWaitResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "agents/wait/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents wait response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsWaitResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "agents/wait/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents wait response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandApiConfigAddressGetResponseSchema = z.object({
  address: z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.api.config.address.get.Response" });

// src/viewer/command/api/config/address/get.ts
async function apiConfigAddressGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/address/get" }), z.union([CliErrorSchema, CliCommandApiConfigAddressGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config address get: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigAddressGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/address/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config address get: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigAddressGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/address/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config address get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigAddressGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/address/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config address get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigAddressGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/address/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config address get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigAddressGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/address/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config address get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigAddressSetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "api/config/address/set" }), z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config address set: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigAddressSetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "api/config/address/set" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config address set: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigAddressSetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/address/set/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config address set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigAddressSetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/address/set/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config address set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigAddressSetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/address/set/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config address set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigAddressSetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/address/set/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config address set response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandApiConfigCommitAuthorEmailGetResponseSchema = z.object({
  commit_author_email: z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.api.config.commit_author_email.get.Response" });

// src/viewer/command/api/config/commit_author_email/get.ts
async function apiConfigCommitAuthorEmailGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/commit_author_email/get" }), z.union([CliErrorSchema, CliCommandApiConfigCommitAuthorEmailGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config commit_author_email get: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigCommitAuthorEmailGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/commit_author_email/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config commit_author_email get: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigCommitAuthorEmailGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/commit_author_email/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config commit_author_email get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigCommitAuthorEmailGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/commit_author_email/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config commit_author_email get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigCommitAuthorEmailGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/commit_author_email/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config commit_author_email get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigCommitAuthorEmailGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/commit_author_email/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config commit_author_email get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigCommitAuthorEmailSetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "api/config/commit_author_email/set" }), z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config commit_author_email set: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigCommitAuthorEmailSetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "api/config/commit_author_email/set" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config commit_author_email set: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigCommitAuthorEmailSetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/commit_author_email/set/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config commit_author_email set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigCommitAuthorEmailSetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/commit_author_email/set/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config commit_author_email set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigCommitAuthorEmailSetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/commit_author_email/set/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config commit_author_email set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigCommitAuthorEmailSetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/commit_author_email/set/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config commit_author_email set response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandApiConfigCommitAuthorNameGetResponseSchema = z.object({
  commit_author_name: z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.api.config.commit_author_name.get.Response" });

// src/viewer/command/api/config/commit_author_name/get.ts
async function apiConfigCommitAuthorNameGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/commit_author_name/get" }), z.union([CliErrorSchema, CliCommandApiConfigCommitAuthorNameGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config commit_author_name get: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigCommitAuthorNameGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/commit_author_name/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config commit_author_name get: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigCommitAuthorNameGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/commit_author_name/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config commit_author_name get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigCommitAuthorNameGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/commit_author_name/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config commit_author_name get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigCommitAuthorNameGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/commit_author_name/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config commit_author_name get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigCommitAuthorNameGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/commit_author_name/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config commit_author_name get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigCommitAuthorNameSetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "api/config/commit_author_name/set" }), z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config commit_author_name set: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigCommitAuthorNameSetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "api/config/commit_author_name/set" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config commit_author_name set: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigCommitAuthorNameSetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/commit_author_name/set/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config commit_author_name set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigCommitAuthorNameSetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/commit_author_name/set/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config commit_author_name set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigCommitAuthorNameSetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/commit_author_name/set/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config commit_author_name set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigCommitAuthorNameSetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/commit_author_name/set/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config commit_author_name set response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandApiConfigGetResponseSchema = z.object({
  address: z.string().nullable().meta({ omitempty: true }).optional(),
  commit_author_email: z.string().nullable().meta({ omitempty: true }).optional(),
  commit_author_name: z.string().nullable().meta({ omitempty: true }).optional(),
  github_authorization: z.string().nullable().meta({ omitempty: true }).optional(),
  http_referer: z.string().nullable().meta({ omitempty: true }).optional(),
  mcp_authorization: z.record(z.string(), z.string()).nullable().meta({ omitempty: true }).optional(),
  objectiveai_authorization: z.string().nullable().meta({ omitempty: true }).optional(),
  openrouter_authorization: z.string().nullable().meta({ omitempty: true }).optional(),
  user_agent: z.string().nullable().meta({ omitempty: true }).optional(),
  x_title: z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.api.config.get.Response" });

// src/viewer/command/api/config/get.ts
async function apiConfigGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/get" }), z.union([CliErrorSchema, CliCommandApiConfigGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config get: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config get: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandApiConfigGithubAuthorizationGetResponseSchema = z.object({
  github_authorization: z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.api.config.github_authorization.get.Response" });

// src/viewer/command/api/config/github_authorization/get.ts
async function apiConfigGithubAuthorizationGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/github_authorization/get" }), z.union([CliErrorSchema, CliCommandApiConfigGithubAuthorizationGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config github_authorization get: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigGithubAuthorizationGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/github_authorization/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config github_authorization get: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigGithubAuthorizationGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/github_authorization/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config github_authorization get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigGithubAuthorizationGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/github_authorization/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config github_authorization get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigGithubAuthorizationGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/github_authorization/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config github_authorization get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigGithubAuthorizationGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/github_authorization/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config github_authorization get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigGithubAuthorizationSetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "api/config/github_authorization/set" }), z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config github_authorization set: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigGithubAuthorizationSetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "api/config/github_authorization/set" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config github_authorization set: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigGithubAuthorizationSetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/github_authorization/set/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config github_authorization set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigGithubAuthorizationSetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/github_authorization/set/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config github_authorization set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigGithubAuthorizationSetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/github_authorization/set/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config github_authorization set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigGithubAuthorizationSetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/github_authorization/set/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config github_authorization set response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandApiConfigHttpRefererGetResponseSchema = z.object({
  http_referer: z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.api.config.http_referer.get.Response" });

// src/viewer/command/api/config/http_referer/get.ts
async function apiConfigHttpRefererGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/http_referer/get" }), z.union([CliErrorSchema, CliCommandApiConfigHttpRefererGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config http_referer get: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigHttpRefererGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/http_referer/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config http_referer get: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigHttpRefererGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/http_referer/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config http_referer get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigHttpRefererGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/http_referer/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config http_referer get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigHttpRefererGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/http_referer/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config http_referer get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigHttpRefererGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/http_referer/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config http_referer get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigHttpRefererSetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "api/config/http_referer/set" }), z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config http_referer set: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigHttpRefererSetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "api/config/http_referer/set" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config http_referer set: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigHttpRefererSetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/http_referer/set/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config http_referer set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigHttpRefererSetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/http_referer/set/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config http_referer set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigHttpRefererSetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/http_referer/set/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config http_referer set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigHttpRefererSetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/http_referer/set/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config http_referer set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigMcpAuthorizationAddExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "api/config/mcp_authorization/add" }), z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config mcp_authorization add: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigMcpAuthorizationAddExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "api/config/mcp_authorization/add" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config mcp_authorization add: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigMcpAuthorizationAddRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/mcp_authorization/add/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config mcp_authorization add request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigMcpAuthorizationAddRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/mcp_authorization/add/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config mcp_authorization add request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigMcpAuthorizationAddResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/mcp_authorization/add/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config mcp_authorization add response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigMcpAuthorizationAddResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/mcp_authorization/add/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config mcp_authorization add response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigMcpAuthorizationDelExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "api/config/mcp_authorization/del" }), z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config mcp_authorization del: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigMcpAuthorizationDelExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "api/config/mcp_authorization/del" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config mcp_authorization del: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigMcpAuthorizationDelRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/mcp_authorization/del/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config mcp_authorization del request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigMcpAuthorizationDelRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/mcp_authorization/del/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config mcp_authorization del request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigMcpAuthorizationDelResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/mcp_authorization/del/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config mcp_authorization del response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigMcpAuthorizationDelResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/mcp_authorization/del/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config mcp_authorization del response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandApiConfigMcpAuthorizationGetResponseSchema = z.object({
  mcp_authorization: z.record(z.string(), z.string()).nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.api.config.mcp_authorization.get.Response" });

// src/viewer/command/api/config/mcp_authorization/get.ts
async function apiConfigMcpAuthorizationGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/mcp_authorization/get" }), z.union([CliErrorSchema, CliCommandApiConfigMcpAuthorizationGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config mcp_authorization get: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigMcpAuthorizationGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/mcp_authorization/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config mcp_authorization get: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigMcpAuthorizationGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/mcp_authorization/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config mcp_authorization get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigMcpAuthorizationGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/mcp_authorization/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config mcp_authorization get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigMcpAuthorizationGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/mcp_authorization/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config mcp_authorization get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigMcpAuthorizationGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/mcp_authorization/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config mcp_authorization get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandApiConfigObjectiveaiAuthorizationGetResponseSchema = z.object({
  objectiveai_authorization: z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.api.config.objectiveai_authorization.get.Response" });

// src/viewer/command/api/config/objectiveai_authorization/get.ts
async function apiConfigObjectiveaiAuthorizationGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/objectiveai_authorization/get" }), z.union([CliErrorSchema, CliCommandApiConfigObjectiveaiAuthorizationGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config objectiveai_authorization get: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigObjectiveaiAuthorizationGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/objectiveai_authorization/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config objectiveai_authorization get: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigObjectiveaiAuthorizationGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/objectiveai_authorization/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config objectiveai_authorization get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigObjectiveaiAuthorizationGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/objectiveai_authorization/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config objectiveai_authorization get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigObjectiveaiAuthorizationGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/objectiveai_authorization/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config objectiveai_authorization get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigObjectiveaiAuthorizationGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/objectiveai_authorization/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config objectiveai_authorization get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigObjectiveaiAuthorizationSetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "api/config/objectiveai_authorization/set" }), z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config objectiveai_authorization set: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigObjectiveaiAuthorizationSetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "api/config/objectiveai_authorization/set" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config objectiveai_authorization set: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigObjectiveaiAuthorizationSetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/objectiveai_authorization/set/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config objectiveai_authorization set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigObjectiveaiAuthorizationSetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/objectiveai_authorization/set/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config objectiveai_authorization set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigObjectiveaiAuthorizationSetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/objectiveai_authorization/set/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config objectiveai_authorization set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigObjectiveaiAuthorizationSetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/objectiveai_authorization/set/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config objectiveai_authorization set response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandApiConfigOpenrouterAuthorizationGetResponseSchema = z.object({
  openrouter_authorization: z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.api.config.openrouter_authorization.get.Response" });

// src/viewer/command/api/config/openrouter_authorization/get.ts
async function apiConfigOpenrouterAuthorizationGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/openrouter_authorization/get" }), z.union([CliErrorSchema, CliCommandApiConfigOpenrouterAuthorizationGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config openrouter_authorization get: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigOpenrouterAuthorizationGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/openrouter_authorization/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config openrouter_authorization get: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigOpenrouterAuthorizationGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/openrouter_authorization/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config openrouter_authorization get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigOpenrouterAuthorizationGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/openrouter_authorization/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config openrouter_authorization get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigOpenrouterAuthorizationGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/openrouter_authorization/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config openrouter_authorization get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigOpenrouterAuthorizationGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/openrouter_authorization/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config openrouter_authorization get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigOpenrouterAuthorizationSetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "api/config/openrouter_authorization/set" }), z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config openrouter_authorization set: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigOpenrouterAuthorizationSetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "api/config/openrouter_authorization/set" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config openrouter_authorization set: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigOpenrouterAuthorizationSetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/openrouter_authorization/set/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config openrouter_authorization set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigOpenrouterAuthorizationSetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/openrouter_authorization/set/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config openrouter_authorization set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigOpenrouterAuthorizationSetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/openrouter_authorization/set/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config openrouter_authorization set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigOpenrouterAuthorizationSetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/openrouter_authorization/set/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config openrouter_authorization set response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandApiConfigUserAgentGetResponseSchema = z.object({
  user_agent: z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.api.config.user_agent.get.Response" });

// src/viewer/command/api/config/user_agent/get.ts
async function apiConfigUserAgentGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/user_agent/get" }), z.union([CliErrorSchema, CliCommandApiConfigUserAgentGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config user_agent get: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigUserAgentGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/user_agent/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config user_agent get: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigUserAgentGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/user_agent/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config user_agent get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigUserAgentGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/user_agent/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config user_agent get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigUserAgentGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/user_agent/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config user_agent get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigUserAgentGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/user_agent/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config user_agent get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigUserAgentSetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "api/config/user_agent/set" }), z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config user_agent set: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigUserAgentSetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "api/config/user_agent/set" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config user_agent set: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigUserAgentSetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/user_agent/set/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config user_agent set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigUserAgentSetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/user_agent/set/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config user_agent set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigUserAgentSetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/user_agent/set/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config user_agent set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigUserAgentSetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/user_agent/set/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config user_agent set response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandApiConfigXTitleGetResponseSchema = z.object({
  x_title: z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.api.config.x_title.get.Response" });

// src/viewer/command/api/config/x_title/get.ts
async function apiConfigXTitleGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/x_title/get" }), z.union([CliErrorSchema, CliCommandApiConfigXTitleGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config x_title get: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigXTitleGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/x_title/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config x_title get: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigXTitleGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/x_title/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config x_title get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigXTitleGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/x_title/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config x_title get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigXTitleGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/x_title/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config x_title get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigXTitleGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/x_title/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config x_title get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigXTitleSetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "api/config/x_title/set" }), z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config x_title set: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigXTitleSetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "api/config/x_title/set" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config x_title set: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigXTitleSetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/x_title/set/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config x_title set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigXTitleSetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/x_title/set/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config x_title set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigXTitleSetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/config/x_title/set/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config x_title set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiConfigXTitleSetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/config/x_title/set/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api config x_title set response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandApiKillResponseSchema = z.object({
  killed: z.number().int().min(0).max(4294967295)
}).meta({ title: "cli.command.api.kill.Response" });

// src/viewer/command/api/kill.ts
async function apiKillExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/kill" }), z.union([CliErrorSchema, CliCommandApiKillResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api kill: cli produced no output before the end marker");
  }
  return first;
}
async function apiKillExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/kill" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api kill: cli produced no output before the end marker");
  }
  return first;
}
async function apiKillRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/kill/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api kill request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiKillRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/kill/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api kill request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiKillResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/kill/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api kill response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiKillResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/kill/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api kill response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandApiSpawnResponseSchema = z.object({
  listening: z.string()
}).meta({ title: "cli.command.api.spawn.Response" });

// src/viewer/command/api/spawn.ts
async function apiSpawnExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/spawn" }), z.union([CliErrorSchema, CliCommandApiSpawnResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api spawn: cli produced no output before the end marker");
  }
  return first;
}
async function apiSpawnExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/spawn" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api spawn: cli produced no output before the end marker");
  }
  return first;
}
async function apiSpawnRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/spawn/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api spawn request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiSpawnRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/spawn/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api spawn request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiSpawnResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "api/spawn/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api spawn response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function apiSpawnResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "api/spawn/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("api spawn response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandDbConfigAddressGetResponseSchema = z.object({
  address: z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.db.config.address.get.Response" });

// src/viewer/command/db/config/address/get.ts
async function dbConfigAddressGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/config/address/get" }), z.union([CliErrorSchema, CliCommandDbConfigAddressGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config address get: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigAddressGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/config/address/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config address get: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigAddressGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/config/address/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config address get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigAddressGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/config/address/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config address get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigAddressGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/config/address/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config address get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigAddressGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/config/address/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config address get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigAddressSetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "db/config/address/set" }), z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config address set: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigAddressSetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "db/config/address/set" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config address set: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigAddressSetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/config/address/set/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config address set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigAddressSetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/config/address/set/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config address set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigAddressSetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/config/address/set/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config address set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigAddressSetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/config/address/set/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config address set response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandDbConfigDatabaseGetResponseSchema = z.object({
  database: z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.db.config.database.get.Response" });

// src/viewer/command/db/config/database/get.ts
async function dbConfigDatabaseGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/config/database/get" }), z.union([CliErrorSchema, CliCommandDbConfigDatabaseGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config database get: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigDatabaseGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/config/database/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config database get: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigDatabaseGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/config/database/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config database get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigDatabaseGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/config/database/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config database get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigDatabaseGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/config/database/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config database get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigDatabaseGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/config/database/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config database get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigDatabaseSetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "db/config/database/set" }), z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config database set: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigDatabaseSetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "db/config/database/set" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config database set: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigDatabaseSetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/config/database/set/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config database set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigDatabaseSetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/config/database/set/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config database set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigDatabaseSetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/config/database/set/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config database set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigDatabaseSetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/config/database/set/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config database set response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandDbConfigGetResponseSchema = z.object({
  address: z.string().nullable().meta({ omitempty: true }).optional(),
  database: z.string().nullable().meta({ omitempty: true }).optional(),
  password: z.string().nullable().meta({ omitempty: true }).optional(),
  user: z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.db.config.get.Response" });

// src/viewer/command/db/config/get.ts
async function dbConfigGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/config/get" }), z.union([CliErrorSchema, CliCommandDbConfigGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config get: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/config/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config get: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/config/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/config/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/config/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/config/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandDbConfigPasswordGetResponseSchema = z.object({
  password: z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.db.config.password.get.Response" });

// src/viewer/command/db/config/password/get.ts
async function dbConfigPasswordGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/config/password/get" }), z.union([CliErrorSchema, CliCommandDbConfigPasswordGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config password get: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigPasswordGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/config/password/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config password get: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigPasswordGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/config/password/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config password get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigPasswordGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/config/password/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config password get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigPasswordGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/config/password/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config password get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigPasswordGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/config/password/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config password get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigPasswordSetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "db/config/password/set" }), z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config password set: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigPasswordSetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "db/config/password/set" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config password set: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigPasswordSetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/config/password/set/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config password set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigPasswordSetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/config/password/set/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config password set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigPasswordSetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/config/password/set/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config password set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigPasswordSetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/config/password/set/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config password set response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandDbConfigUserGetResponseSchema = z.object({
  user: z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.db.config.user.get.Response" });

// src/viewer/command/db/config/user/get.ts
async function dbConfigUserGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/config/user/get" }), z.union([CliErrorSchema, CliCommandDbConfigUserGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config user get: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigUserGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/config/user/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config user get: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigUserGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/config/user/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config user get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigUserGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/config/user/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config user get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigUserGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/config/user/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config user get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigUserGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/config/user/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config user get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigUserSetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "db/config/user/set" }), z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config user set: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigUserSetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "db/config/user/set" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config user set: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigUserSetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/config/user/set/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config user set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigUserSetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/config/user/set/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config user set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigUserSetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/config/user/set/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config user set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbConfigUserSetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/config/user/set/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db config user set response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandDbKillResponseSchema = z.object({
  killed: z.number().int().min(0).max(4294967295)
}).meta({ title: "cli.command.db.kill.Response" });

// src/viewer/command/db/kill.ts
async function dbKillExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/kill" }), z.union([CliErrorSchema, CliCommandDbKillResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db kill: cli produced no output before the end marker");
  }
  return first;
}
async function dbKillExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/kill" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db kill: cli produced no output before the end marker");
  }
  return first;
}
async function dbKillRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/kill/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db kill request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbKillRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/kill/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db kill request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbKillResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/kill/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db kill response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbKillResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/kill/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db kill response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandDbQueryColumnSchema = z.object({
  name: z.string(),
  type: z.string()
}).describe('One result column. `r#type` is the Postgres `pg_type.typname`\n(e.g. `"int8"`, `"text"`, `"jsonb"`, `"timestamptz"`). Callers\nneeding precision/scale/array-element-type can inspect the\nname; richer typeinfo is deferred.').meta({ title: "cli.command.db.query.Column" });

// src/cli/command/db/query/response.ts
var CliCommandDbQueryResponseSchema = z.object({
  columns: z.array(CliCommandDbQueryColumnSchema).describe("Result columns in select-list order. Empty for no-row\nstatements (SET / LISTEN / DO)."),
  command_tag: z.string().describe('Wire-protocol-style command tag \u2014 e.g. `"SELECT 5"`,\n`"SHOW 1"`, `"EXPLAIN 12"`, `"SET 0"`. Synthesized from\n`{leading_keyword} {row_count}` since sqlx 0.8 doesn\'t\nexpose the wire-protocol `CommandComplete` tag; close\nenough for telemetry / display.'),
  rows: z.array(z.array(JsonValueSchema)).describe("One inner Vec per row, length matches `columns.len()`.\nEach cell is a `serde_json::Value` produced by the CLI's\nper-cell `pg_value_to_json` decoder. Common type\nencodings: text/uuid/timestamps as JSON String, numeric\nas String (preserving precision), bytea as base64 String,\njson/jsonb passthrough as Value, arrays recurse. Empty\nwhen the statement returns no rows (no-row command OR a\nSELECT with no matches)."),
  truncated: z.boolean().describe('Always `false` in the current design. Reserved on the wire\nso a future "soft truncation" mode can be added without a\nshape break.')
}).describe("Unary response from `db query`.").meta({ title: "cli.command.db.query.Response" });

// src/viewer/command/db/query.ts
async function dbQueryExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/query" }), z.union([CliErrorSchema, CliCommandDbQueryResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db query: cli produced no output before the end marker");
  }
  return first;
}
async function dbQueryExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/query" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db query: cli produced no output before the end marker");
  }
  return first;
}
async function dbQueryRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/query/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db query request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbQueryRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/query/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db query request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbQueryResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/query/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db query response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbQueryResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/query/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db query response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandDbSpawnResponseSchema = z.object({
  listening: z.string()
}).meta({ title: "cli.command.db.spawn.Response" });

// src/viewer/command/db/spawn.ts
async function dbSpawnExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/spawn" }), z.union([CliErrorSchema, CliCommandDbSpawnResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db spawn: cli produced no output before the end marker");
  }
  return first;
}
async function dbSpawnExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/spawn" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db spawn: cli produced no output before the end marker");
  }
  return first;
}
async function dbSpawnRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/spawn/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db spawn request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbSpawnRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/spawn/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db spawn request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbSpawnResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "db/spawn/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db spawn response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function dbSpawnResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "db/spawn/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("db spawn response_schema: cli produced no output before the end marker");
  }
  return first;
}
var FunctionsExpressionTaskOutputSchema = z.union([z.number().min(-34028234663852886e22).max(34028234663852886e22).describe("A single scalar score.").meta({ "variantTitle": "Scalar" }), z.array(z.number().min(-34028234663852886e22).max(34028234663852886e22)).describe("A vector of scores.").meta({ "variantTitle": "Vector" }), z.array(z.array(z.number().min(-34028234663852886e22).max(34028234663852886e22))).describe("Multiple vectors of scores (from mapped tasks).").meta({ "variantTitle": "Vectors" }), z.object({
  error: JsonValueSchema
}).describe("An error occurred during execution.").meta({ "variantTitle": "Err" })]).describe("Owned task output variants.").meta({ title: "functions.expression.TaskOutput" });

// src/functions/executions/response/output.ts
var FunctionsExecutionsResponseOutputSchema = z.object({
  output: FunctionsExpressionTaskOutputSchema
}).describe("Wrapper for function execution output, distinguishing between\na null output value and a missing output.").meta({ title: "functions.executions.response.Output" });
var FunctionsExecutionsResponseStreamingObjectSchema = z.enum(["scalar.function.execution.chunk", "vector.function.execution.chunk"]).meta({ title: "functions.executions.response.streaming.Object" });
var FunctionsExecutionsResponseStreamingReasoningSummaryChunkSchema = z.object({
  agent_full_id: z.string().describe("WF-level id: concatenation of the primary agent's id with all\nfallback ids (see `InlineAgentWithFallbacks::full_id`). Same\nfor every slot in the same WF request."),
  agent_id: z.string().describe("Leaf agent id of the slot that produced this chunk. For the\nprimary attempt this is the primary agent's id; on fallback it\nis the fallback agent's id. Same on every chunk of a slot."),
  agent_instance_hierarchy: z.string().describe("Full agent instance hierarchy for this completion's slot \u2014\n`{ctx lineage}/{agent_full_id}-{response_id}`, or the fixed\ncontinuation value on resume. Same on every chunk of a slot."),
  agent_remote: RemotePathSchema.nullable().describe("`RemotePath` the WF was fetched from. `None` when the WF was\nsupplied inline. Same for every slot in the same WF request.").meta({ omitempty: true }).optional(),
  continuation: z.string().nullable().describe("Continuation state for multi-turn conversations (only present in the final chunk).").meta({ omitempty: true }).optional(),
  created: z.number().int().min(0).max(18446744073709552e3),
  error: ErrorResponseErrorSchema.nullable().meta({ omitempty: true }).optional(),
  id: z.string(),
  messages: z.array(AgentCompletionsResponseStreamingMessageChunkSchema),
  messages_queued: z.boolean().nullable().describe("`true` when the MCP proxy holds queued messages that were not\ndelivered to the agent via a tool response on this turn. Only\nset when `continuation` is also set \u2014 the caller acts on it by\nissuing the continuation. Absent when nothing is queued, when\nthere is no continuation to act on, or when the peek failed\n(the failure is surfaced via `error`).").meta({ omitempty: true }).optional(),
  object: AgentCompletionsResponseStreamingObjectSchema.describe('The object type (always "agent.completion.chunk").'),
  upstream: AgentUpstreamSchema.describe("Upstream provider"),
  usage: AgentCompletionsResponseUsageSchema.nullable().describe("Token usage (only present in the final chunk).").meta({ omitempty: true }).optional()
}).describe("A chunk of a streaming agent completion response.\n\nMultiple chunks are received via Server-Sent Events and can be\naccumulated into a complete [`AgentCompletion`](response::unary::AgentCompletion)\nusing the [`push`](Self::push) method.").meta({ title: "functions.executions.response.streaming.ReasoningSummaryChunk" });
var FunctionsExecutionsResponseStreamingFunctionExecutionTaskChunkSchema = z.object({
  created: z.number().int().min(0).max(18446744073709552e3),
  error: ErrorResponseErrorSchema.nullable().meta({ omitempty: true }).optional(),
  function: RemotePathSchema.nullable().optional(),
  id: z.string(),
  index: z.number().int().min(0).max(18446744073709552e3),
  object: FunctionsExecutionsResponseStreamingObjectSchema,
  output: FunctionsExecutionsResponseOutputSchema.nullable().meta({ omitempty: true }).optional(),
  profile: RemotePathSchema.nullable().optional(),
  reasoning: FunctionsExecutionsResponseStreamingReasoningSummaryChunkSchema.nullable().meta({ omitempty: true }).optional(),
  split_index: z.number().int().min(0).max(18446744073709552e3).nullable().meta({ omitempty: true }).optional(),
  swiss_pool_index: z.number().int().min(0).max(18446744073709552e3).nullable().meta({ omitempty: true }).optional(),
  swiss_round: z.number().int().min(0).max(18446744073709552e3).nullable().meta({ omitempty: true }).optional(),
  task_index: z.number().int().min(0).max(18446744073709552e3),
  task_path: z.array(z.number().int().min(0).max(18446744073709552e3)),
  tasks: z.array(z.lazy(() => FunctionsExecutionsResponseStreamingTaskChunkSchema)),
  tasks_errors: z.boolean().nullable().meta({ omitempty: true }).optional(),
  usage: AgentCompletionsResponseUsageSchema.nullable().meta({ omitempty: true }).optional()
}).meta({ title: "functions.executions.response.streaming.FunctionExecutionTaskChunk" });
var VectorCompletionsResponseStreamingAgentCompletionChunkSchema = z.object({
  agent_full_id: z.string().describe("WF-level id: concatenation of the primary agent's id with all\nfallback ids (see `InlineAgentWithFallbacks::full_id`). Same\nfor every slot in the same WF request."),
  agent_id: z.string().describe("Leaf agent id of the slot that produced this chunk. For the\nprimary attempt this is the primary agent's id; on fallback it\nis the fallback agent's id. Same on every chunk of a slot."),
  agent_instance_hierarchy: z.string().describe("Full agent instance hierarchy for this completion's slot \u2014\n`{ctx lineage}/{agent_full_id}-{response_id}`, or the fixed\ncontinuation value on resume. Same on every chunk of a slot."),
  agent_remote: RemotePathSchema.nullable().describe("`RemotePath` the WF was fetched from. `None` when the WF was\nsupplied inline. Same for every slot in the same WF request.").meta({ omitempty: true }).optional(),
  continuation: z.string().nullable().describe("Continuation state for multi-turn conversations (only present in the final chunk).").meta({ omitempty: true }).optional(),
  created: z.number().int().min(0).max(18446744073709552e3),
  error: ErrorResponseErrorSchema.nullable().describe("Error details if this completion failed.").meta({ omitempty: true }).optional(),
  id: z.string(),
  index: z.number().int().min(0).max(18446744073709552e3).describe("Index used to correlate chunks from the same completion."),
  messages: z.array(AgentCompletionsResponseStreamingMessageChunkSchema),
  messages_queued: z.boolean().nullable().describe("`true` when the MCP proxy holds queued messages that were not\ndelivered to the agent via a tool response on this turn. Only\nset when `continuation` is also set \u2014 the caller acts on it by\nissuing the continuation. Absent when nothing is queued, when\nthere is no continuation to act on, or when the peek failed\n(the failure is surfaced via `error`).").meta({ omitempty: true }).optional(),
  object: AgentCompletionsResponseStreamingObjectSchema.describe('The object type (always "agent.completion.chunk").'),
  upstream: AgentUpstreamSchema.describe("Upstream provider"),
  usage: AgentCompletionsResponseUsageSchema.nullable().describe("Token usage (only present in the final chunk).").meta({ omitempty: true }).optional()
}).describe("A streaming agent completion chunk from a single agent within a vector completion.\n\nThe `index` field is used to correlate chunks belonging to the same\nunderlying completion when accumulating via [`push`](Self::push).").meta({ title: "vector.completions.response.streaming.AgentCompletionChunk" });
var VectorCompletionsResponseStreamingObjectSchema = z.literal("vector.completion.chunk").describe("A streaming vector completion chunk.").meta({ title: "vector.completions.response.streaming.Object" });
var VectorCompletionsResponseVoteSchema = z.object({
  agent_full_id: z.string().describe("WF-level id of the agent that produced this vote \u2014 concatenation\nof the primary agent's id with all fallback ids (see\n`InlineAgentWithFallbacks::full_id`). Same for every slot in the\nsame WF request and deterministic across api processes."),
  agent_id: z.string().describe("Leaf agent id of the slot that produced this vote (matches the\n`agent_id` on the corresponding\n[`super::super::super::super::agent::completions::response::unary::AgentCompletion`]).\nWhen fallbacks fired, this is the fallback's id rather than the\nprimary's."),
  flat_swarm_index: z.number().int().min(0).max(18446744073709552e3).describe("Flattened index accounting for agent counts in the swarm."),
  prompt_id: z.string().describe("Content hash of the request messages (for caching/deduplication)."),
  responses_ids: z.array(z.string()).describe("Content hashes of each response option in the request."),
  swarm_index: z.number().int().min(0).max(18446744073709552e3).describe("Index of the agent configuration within the swarm."),
  vote: z.array(z.number().min(-34028234663852886e22).max(34028234663852886e22)).describe("The vote distribution. Each index corresponds to a response from the\nrequest. Typically one element is 1.0 (selected) and the rest are 0.0."),
  weight: z.number().min(-34028234663852886e22).max(34028234663852886e22).describe("The weight applied to this vote when computing final scores.")
}).describe("A single LLM's vote in a vector completion.\n\nEach LLM in the swarm produces a vote indicating which response(s) it\nselected. Votes are weighted according to the profile and combined to\nproduce the final scores.\n\n# Vote Format\n\nThe `vote` field is a vector of decimals corresponding to the responses\nin the request. Typically one element is 1.0 and the rest are 0.0 (discrete\nselection), but when `top_logprobs` is used, votes may be probability\ndistributions.").meta({ title: "vector.completions.response.Vote" });

// src/functions/executions/response/streaming/vectorCompletionTaskChunk.ts
var FunctionsExecutionsResponseStreamingVectorCompletionTaskChunkSchema = z.object({
  completions: z.array(VectorCompletionsResponseStreamingAgentCompletionChunkSchema).describe("Incremental agent completion chunks from each agent."),
  created: z.number().int().min(0).max(18446744073709552e3).describe("Unix timestamp when the completion was created."),
  error: ErrorResponseErrorSchema.nullable().meta({ omitempty: true }).optional(),
  id: z.string().describe("Unique identifier for this vector completion."),
  index: z.number().int().min(0).max(18446744073709552e3),
  object: VectorCompletionsResponseStreamingObjectSchema.describe('Object type identifier (`"vector.completion.chunk"`).'),
  scores: z.array(z.number().min(-34028234663852886e22).max(34028234663852886e22)).describe("Current weighted scores. Updated as new votes arrive."),
  swarm: z.string().describe("ID of the swarm used for this completion."),
  task_index: z.number().int().min(0).max(18446744073709552e3),
  task_path: z.array(z.number().int().min(0).max(18446744073709552e3)),
  usage: AgentCompletionsResponseUsageSchema.nullable().describe("Aggregated usage statistics. Typically present only in the final chunk.").meta({ omitempty: true }).optional(),
  votes: z.array(VectorCompletionsResponseVoteSchema).describe("Votes received so far. New votes are appended in subsequent chunks."),
  weights: z.array(z.number().min(-34028234663852886e22).max(34028234663852886e22)).describe("Current weight distribution across responses. Updated as new votes arrive.")
}).describe("A chunk in a streaming vector completion response.\n\nEach chunk contains incremental updates to the completion. Use the\n[`push`](Self::push) method to accumulate chunks into a complete response.").meta({ title: "functions.executions.response.streaming.VectorCompletionTaskChunk" });

// src/functions/executions/response/streaming/taskChunk.ts
var FunctionsExecutionsResponseStreamingTaskChunkSchema = z.union([z.lazy(() => FunctionsExecutionsResponseStreamingFunctionExecutionTaskChunkSchema).meta({ "title": "functions.executions.response.streaming.FunctionExecutionTaskChunk", "variantTitle": "FunctionExecution" }), FunctionsExecutionsResponseStreamingVectorCompletionTaskChunkSchema.meta({ "title": "functions.executions.response.streaming.VectorCompletionTaskChunk", "variantTitle": "VectorCompletion" })]).meta({ title: "functions.executions.response.streaming.TaskChunk" });

// src/functions/executions/response/streaming/functionExecutionChunk.ts
var FunctionsExecutionsResponseStreamingFunctionExecutionChunkSchema = z.object({
  created: z.number().int().min(0).max(18446744073709552e3),
  error: ErrorResponseErrorSchema.nullable().meta({ omitempty: true }).optional(),
  function: RemotePathSchema.nullable().optional(),
  id: z.string(),
  object: FunctionsExecutionsResponseStreamingObjectSchema,
  output: FunctionsExecutionsResponseOutputSchema.nullable().meta({ omitempty: true }).optional(),
  profile: RemotePathSchema.nullable().optional(),
  reasoning: FunctionsExecutionsResponseStreamingReasoningSummaryChunkSchema.nullable().meta({ omitempty: true }).optional(),
  tasks: z.array(FunctionsExecutionsResponseStreamingTaskChunkSchema),
  tasks_errors: z.boolean().nullable().meta({ omitempty: true }).optional(),
  usage: AgentCompletionsResponseUsageSchema.nullable().meta({ omitempty: true }).optional()
}).meta({ title: "functions.executions.response.streaming.FunctionExecutionChunk" });

// src/cli/command/functions/execute/standard/responseItem.ts
var CliCommandFunctionsExecuteStandardResponseItemSchema = z.union([FunctionsExecutionsResponseStreamingFunctionExecutionChunkSchema.meta({ "title": "functions.executions.response.streaming.FunctionExecutionChunk", "variantTitle": "Chunk" }), z.string().meta({ "variantTitle": "Id" })]).meta({ title: "cli.command.functions.execute.standard.ResponseItem" });

// src/viewer/command/functions/execute/standard.ts
function functionsExecuteStandardExecuteStreaming(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, dangerous_advanced: { ...request.dangerous_advanced ?? {}, stream: true }, path_type: "functions/execute/standard" }), z.union([CliErrorSchema, CliCommandFunctionsExecuteStandardResponseItemSchema]));
}
function functionsExecuteStandardExecuteStreamingTransform(request, transform) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, dangerous_advanced: { ...request.dangerous_advanced ?? {}, stream: true }, path_type: "functions/execute/standard" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function functionsExecuteStandardExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, dangerous_advanced: request.dangerous_advanced ? { ...request.dangerous_advanced, stream: void 0 } : request.dangerous_advanced, path_type: "functions/execute/standard" }), z.union([CliErrorSchema, z.string()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions execute standard: cli produced no output before the end marker");
  }
  return first;
}
async function functionsExecuteStandardExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, dangerous_advanced: request.dangerous_advanced ? { ...request.dangerous_advanced, stream: void 0 } : request.dangerous_advanced, path_type: "functions/execute/standard" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions execute standard: cli produced no output before the end marker");
  }
  return first;
}
async function functionsExecuteStandardRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "functions/execute/standard/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions execute standard request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsExecuteStandardRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "functions/execute/standard/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions execute standard request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsExecuteStandardResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "functions/execute/standard/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions execute standard response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsExecuteStandardResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "functions/execute/standard/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions execute standard response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandFunctionsExecuteSwissSystemResponseItemSchema = z.union([FunctionsExecutionsResponseStreamingFunctionExecutionChunkSchema.meta({ "title": "functions.executions.response.streaming.FunctionExecutionChunk", "variantTitle": "Chunk" }), z.string().meta({ "variantTitle": "Id" })]).meta({ title: "cli.command.functions.execute.swiss_system.ResponseItem" });

// src/viewer/command/functions/execute/swiss_system.ts
function functionsExecuteSwissSystemExecuteStreaming(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, dangerous_advanced: { ...request.dangerous_advanced ?? {}, stream: true }, path_type: "functions/execute/swiss_system" }), z.union([CliErrorSchema, CliCommandFunctionsExecuteSwissSystemResponseItemSchema]));
}
function functionsExecuteSwissSystemExecuteStreamingTransform(request, transform) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, dangerous_advanced: { ...request.dangerous_advanced ?? {}, stream: true }, path_type: "functions/execute/swiss_system" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function functionsExecuteSwissSystemExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, dangerous_advanced: request.dangerous_advanced ? { ...request.dangerous_advanced, stream: void 0 } : request.dangerous_advanced, path_type: "functions/execute/swiss_system" }), z.union([CliErrorSchema, z.string()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions execute swiss_system: cli produced no output before the end marker");
  }
  return first;
}
async function functionsExecuteSwissSystemExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, dangerous_advanced: request.dangerous_advanced ? { ...request.dangerous_advanced, stream: void 0 } : request.dangerous_advanced, path_type: "functions/execute/swiss_system" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions execute swiss_system: cli produced no output before the end marker");
  }
  return first;
}
async function functionsExecuteSwissSystemRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "functions/execute/swiss_system/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions execute swiss_system request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsExecuteSwissSystemRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "functions/execute/swiss_system/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions execute swiss_system request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsExecuteSwissSystemResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "functions/execute/swiss_system/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions execute swiss_system response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsExecuteSwissSystemResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "functions/execute/swiss_system/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions execute swiss_system response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandFunctionsGetResponseSchema = RemotePathSchema.and(z.object({})).describe("Response for `functions get` \u2014 a resolved Function with its remote path.").meta({ title: "cli.command.functions.get.Response" });

// src/viewer/command/functions/get.ts
async function functionsGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "functions/get" }), z.union([CliErrorSchema, CliCommandFunctionsGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions get: cli produced no output before the end marker");
  }
  return first;
}
async function functionsGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "functions/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions get: cli produced no output before the end marker");
  }
  return first;
}
async function functionsGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "functions/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "functions/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "functions/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "functions/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions get response_schema: cli produced no output before the end marker");
  }
  return first;
}
function functionsListExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "functions/list" }), z.union([CliErrorSchema, RemotePathSchema]));
}
function functionsListExecuteTransform(request, transform) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "functions/list" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function functionsListRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "functions/list/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsListRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "functions/list/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsListResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "functions/list/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions list response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsListResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "functions/list/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions list response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandFunctionsProfilesGetResponseSchema = RemotePathSchema.and(z.object({})).describe("Response for `functions profiles get` \u2014 a resolved Profile with its remote path.").meta({ title: "cli.command.functions.profiles.get.Response" });

// src/viewer/command/functions/profiles/get.ts
async function functionsProfilesGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "functions/profiles/get" }), z.union([CliErrorSchema, CliCommandFunctionsProfilesGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles get: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "functions/profiles/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles get: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "functions/profiles/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "functions/profiles/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "functions/profiles/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "functions/profiles/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles get response_schema: cli produced no output before the end marker");
  }
  return first;
}
function functionsProfilesListExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "functions/profiles/list" }), z.union([CliErrorSchema, RemotePathSchema]));
}
function functionsProfilesListExecuteTransform(request, transform) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "functions/profiles/list" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function functionsProfilesListRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "functions/profiles/list/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesListRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "functions/profiles/list/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesListResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "functions/profiles/list/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles list response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesListResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "functions/profiles/list/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles list response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandFunctionsProfilesPublishResponseSchema = z.object({
  sha: z.string()
}).meta({ title: "cli.command.functions.profiles.publish.Response" });

// src/viewer/command/functions/profiles/publish.ts
async function functionsProfilesPublishExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "functions/profiles/publish" }), z.union([CliErrorSchema, CliCommandFunctionsProfilesPublishResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles publish: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesPublishExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "functions/profiles/publish" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles publish: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesPublishRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "functions/profiles/publish/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles publish request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesPublishRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "functions/profiles/publish/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles publish request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesPublishResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "functions/profiles/publish/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles publish response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesPublishResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "functions/profiles/publish/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles publish response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandFunctionsPublishResponseSchema = z.object({
  sha: z.string()
}).meta({ title: "cli.command.functions.publish.Response" });

// src/viewer/command/functions/publish.ts
async function functionsPublishExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "functions/publish" }), z.union([CliErrorSchema, CliCommandFunctionsPublishResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions publish: cli produced no output before the end marker");
  }
  return first;
}
async function functionsPublishExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "functions/publish" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions publish: cli produced no output before the end marker");
  }
  return first;
}
async function functionsPublishRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "functions/publish/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions publish request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsPublishRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "functions/publish/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions publish request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsPublishResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "functions/publish/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions publish response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsPublishResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "functions/publish/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions publish response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandMcpConfigAddressGetResponseSchema = z.object({
  address: z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.mcp.config.address.get.Response" });

// src/viewer/command/mcp/config/address/get.ts
async function mcpConfigAddressGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "mcp/config/address/get" }), z.union([CliErrorSchema, CliCommandMcpConfigAddressGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config address get: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigAddressGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "mcp/config/address/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config address get: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigAddressGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "mcp/config/address/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config address get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigAddressGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "mcp/config/address/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config address get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigAddressGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "mcp/config/address/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config address get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigAddressGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "mcp/config/address/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config address get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigAddressSetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "mcp/config/address/set" }), z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config address set: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigAddressSetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "mcp/config/address/set" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config address set: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigAddressSetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "mcp/config/address/set/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config address set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigAddressSetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "mcp/config/address/set/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config address set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigAddressSetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "mcp/config/address/set/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config address set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigAddressSetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "mcp/config/address/set/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config address set response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandMcpConfigGetResponseSchema = z.object({
  address: z.string().nullable().meta({ omitempty: true }).optional(),
  port: z.number().int().min(0).max(65535).nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.mcp.config.get.Response" });

// src/viewer/command/mcp/config/get.ts
async function mcpConfigGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "mcp/config/get" }), z.union([CliErrorSchema, CliCommandMcpConfigGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config get: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "mcp/config/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config get: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "mcp/config/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "mcp/config/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "mcp/config/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "mcp/config/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandMcpConfigPortGetResponseSchema = z.object({
  port: z.number().int().min(0).max(65535)
}).meta({ title: "cli.command.mcp.config.port.get.Response" });

// src/viewer/command/mcp/config/port/get.ts
async function mcpConfigPortGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "mcp/config/port/get" }), z.union([CliErrorSchema, CliCommandMcpConfigPortGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config port get: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigPortGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "mcp/config/port/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config port get: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigPortGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "mcp/config/port/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config port get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigPortGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "mcp/config/port/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config port get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigPortGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "mcp/config/port/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config port get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigPortGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "mcp/config/port/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config port get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigPortSetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "mcp/config/port/set" }), z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config port set: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigPortSetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "mcp/config/port/set" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config port set: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigPortSetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "mcp/config/port/set/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config port set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigPortSetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "mcp/config/port/set/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config port set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigPortSetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "mcp/config/port/set/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config port set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpConfigPortSetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "mcp/config/port/set/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp config port set response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandMcpKillResponseSchema = z.object({
  killed: z.number().int().min(0).max(4294967295)
}).meta({ title: "cli.command.mcp.kill.Response" });

// src/viewer/command/mcp/kill.ts
async function mcpKillExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "mcp/kill" }), z.union([CliErrorSchema, CliCommandMcpKillResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp kill: cli produced no output before the end marker");
  }
  return first;
}
async function mcpKillExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "mcp/kill" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp kill: cli produced no output before the end marker");
  }
  return first;
}
async function mcpKillRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "mcp/kill/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp kill request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpKillRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "mcp/kill/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp kill request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpKillResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "mcp/kill/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp kill response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpKillResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "mcp/kill/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp kill response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandMcpSpawnResponseSchema = z.object({
  listening: z.string()
}).meta({ title: "cli.command.mcp.spawn.Response" });

// src/viewer/command/mcp/spawn.ts
async function mcpSpawnExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "mcp/spawn" }), z.union([CliErrorSchema, CliCommandMcpSpawnResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp spawn: cli produced no output before the end marker");
  }
  return first;
}
async function mcpSpawnExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "mcp/spawn" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp spawn: cli produced no output before the end marker");
  }
  return first;
}
async function mcpSpawnRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "mcp/spawn/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp spawn request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpSpawnRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "mcp/spawn/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp spawn request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpSpawnResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "mcp/spawn/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp spawn response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpSpawnResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "mcp/spawn/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp spawn response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandPluginsGetResponseMcpServerSchema = z.object({
  authorization: z.boolean(),
  name: z.string()
}).meta({ title: "cli.command.plugins.get.ResponseMcpServer" });
var CliCommandPluginsGetResponseHttpMethodSchema = z.enum(["GET", "POST", "PUT", "PATCH", "DELETE"]).meta({ title: "cli.command.plugins.get.ResponseHttpMethod" });

// src/cli/command/plugins/get/responseViewerRoute.ts
var CliCommandPluginsGetResponseViewerRouteSchema = z.object({
  method: CliCommandPluginsGetResponseHttpMethodSchema,
  path: z.string(),
  type: z.string()
}).meta({ title: "cli.command.plugins.get.ResponseViewerRoute" });
var CliCommandToolsGetExecSchema = z.object({
  linux: z.array(z.string()),
  macos: z.array(z.string()),
  windows: z.array(z.string())
}).describe("Per-OS exec command for a tool. The current platform's vector is\nthe program plus its leading arguments; the caller's `--args` are\nappended, and the result runs with CWD = the tool's version folder's\n`cli/` subdir (`objectiveai.json` lives in the version folder).").meta({ title: "cli.command.tools.get.Exec" });

// src/cli/command/plugins/get/responseManifest.ts
var CliCommandPluginsGetResponseManifestSchema = z.object({
  description: z.string(),
  exec: CliCommandToolsGetExecSchema.describe("Per-OS exec argv for the plugin's cli side, run with CWD =\n`<plugin dir>/cli/` \u2014 the same shape tools use. Required (an\nempty exec is a viewer-only plugin, which still round-trips as\nan empty per-OS object)."),
  mcp_servers: z.array(CliCommandPluginsGetResponseMcpServerSchema),
  name: z.string(),
  owner: z.string(),
  version: z.string(),
  viewer_routes: z.array(CliCommandPluginsGetResponseViewerRouteSchema),
  viewer_url: z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.plugins.get.ResponseManifest" });

// src/viewer/command/plugins/get.ts
async function pluginsGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "plugins/get" }), z.union([CliErrorSchema, CliCommandPluginsGetResponseManifestSchema.nullable()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins get: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "plugins/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins get: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "plugins/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "plugins/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "plugins/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "plugins/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandPluginsInstallFilesystemResponseSchema = z.object({
  instructions: z.string()
}).meta({ title: "cli.command.plugins.install.filesystem.Response" });

// src/viewer/command/plugins/install/filesystem.ts
async function pluginsInstallFilesystemExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "plugins/install/filesystem" }), z.union([CliErrorSchema, CliCommandPluginsInstallFilesystemResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins install filesystem: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsInstallFilesystemExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "plugins/install/filesystem" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins install filesystem: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsInstallFilesystemRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "plugins/install/filesystem/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins install filesystem request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsInstallFilesystemRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "plugins/install/filesystem/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins install filesystem request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsInstallFilesystemResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "plugins/install/filesystem/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins install filesystem response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsInstallFilesystemResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "plugins/install/filesystem/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins install filesystem response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandPluginsInstallGithubResponseSchema = z.object({
  installed: z.boolean()
}).meta({ title: "cli.command.plugins.install.github.Response" });

// src/viewer/command/plugins/install/github.ts
async function pluginsInstallGithubExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "plugins/install/github" }), z.union([CliErrorSchema, CliCommandPluginsInstallGithubResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins install github: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsInstallGithubExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "plugins/install/github" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins install github: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsInstallGithubRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "plugins/install/github/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins install github request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsInstallGithubRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "plugins/install/github/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins install github request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsInstallGithubResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "plugins/install/github/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins install github response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsInstallGithubResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "plugins/install/github/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins install github response_schema: cli produced no output before the end marker");
  }
  return first;
}
function pluginsListExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "plugins/list" }), z.union([CliErrorSchema, CliCommandPluginsGetResponseManifestSchema]));
}
function pluginsListExecuteTransform(request, transform) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "plugins/list" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function pluginsListRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "plugins/list/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsListRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "plugins/list/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsListResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "plugins/list/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins list response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsListResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "plugins/list/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins list response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandPluginsRunMcpTypeSchema = z.literal("mcp").describe('Single-variant discriminator for [`Mcp`]\'s `type` field. Always\n`"mcp"` on the wire.').meta({ title: "cli.command.plugins.run.McpType" });

// src/cli/command/plugins/run/mcp.ts
var CliCommandPluginsRunMcpSchema = z.object({
  type: CliCommandPluginsRunMcpTypeSchema,
  url: z.string()
}).describe('Plugin announces a running MCP server URL. The host routes this\nthrough the standard plugin-notification pipeline and dials the\nURL the same way it would for an entry in the plugin\'s manifest\n`mcp_servers` \u2014 runtime announcements are functionally identical\nto manifest-time declarations.\n\nThe constant `type:"mcp"` discriminator disambiguates this\nvariant from the rest of the untagged [`ResponseItem`] /\n[`crate::cli::plugins::Output`] catch-all, mirroring the\n`type:"error"` discriminator on [`crate::cli::Error`].').meta({ title: "cli.command.plugins.run.Mcp" });

// src/cli/command/plugins/run/responseItem.ts
var CliCommandPluginsRunResponseItemSchema = z.union([CliCommandPluginsRunMcpSchema.meta({ "title": "cli.command.plugins.run.Mcp", "variantTitle": "Mcp" }), CliErrorSchema.meta({ "title": "cli.Error", "variantTitle": "Error" }), JsonValueSchema.meta({ "variantTitle": "Notification" })]).meta({ title: "cli.command.plugins.run.ResponseItem" });

// src/viewer/command/plugins/run.ts
function pluginsRunExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "plugins/run" }), z.union([CliErrorSchema, CliCommandPluginsRunResponseItemSchema]));
}
function pluginsRunExecuteTransform(request, transform) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "plugins/run" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function pluginsRunRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "plugins/run/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins run request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsRunRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "plugins/run/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins run request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsRunResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "plugins/run/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins run response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsRunResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "plugins/run/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins run response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandSwarmsGetResponseSchema = RemotePathSchema.and(z.object({})).describe("Response for `swarms get` \u2014 a Swarm base definition with its remote path.").meta({ title: "cli.command.swarms.get.Response" });

// src/viewer/command/swarms/get.ts
async function swarmsGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "swarms/get" }), z.union([CliErrorSchema, CliCommandSwarmsGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms get: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "swarms/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms get: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "swarms/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "swarms/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "swarms/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "swarms/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms get response_schema: cli produced no output before the end marker");
  }
  return first;
}
function swarmsListExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "swarms/list" }), z.union([CliErrorSchema, RemotePathSchema]));
}
function swarmsListExecuteTransform(request, transform) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "swarms/list" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function swarmsListRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "swarms/list/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsListRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "swarms/list/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsListResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "swarms/list/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms list response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsListResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "swarms/list/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms list response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandSwarmsPublishResponseSchema = z.object({
  sha: z.string()
}).meta({ title: "cli.command.swarms.publish.Response" });

// src/viewer/command/swarms/publish.ts
async function swarmsPublishExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "swarms/publish" }), z.union([CliErrorSchema, CliCommandSwarmsPublishResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms publish: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsPublishExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "swarms/publish" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms publish: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsPublishRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "swarms/publish/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms publish request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsPublishRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "swarms/publish/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms publish request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsPublishResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "swarms/publish/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms publish response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsPublishResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "swarms/publish/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms publish response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandTasksListPluginSchema = z.object({
  owner: z.string(),
  repository: z.string(),
  version: z.string()
}).describe("The plugin that registered a schedule. All three fields are present\ntogether or the whole object is absent (the `schedules` table\nenforces all-or-nothing on its `plugin_*` columns).").meta({ title: "cli.command.tasks.list.Plugin" });

// src/cli/command/tasks/list/responseItem.ts
var CliCommandTasksListResponseItemSchema = z.object({
  agent_instance_hierarchy: z.string(),
  command: z.array(z.string()),
  created_at: z.string().describe("RFC3339 timestamp this schedule row was created."),
  description: z.string(),
  id: z.number().int().min(-9223372036854776e3).max(9223372036854776e3).describe("The `schedules` row id. Monotonic; pass the highest `id` from a\npage as the next request's `after_id` to paginate forward."),
  interval: z.string().nullable().describe('`None` for a oneshot; `Some("30s" / "1h" / "1d12h" / \u2026)`\nfor a recurring schedule, formatted as humantime so the\nlist output reads naturally without a unit-conversion\nstep at the consumer. The CLI parser accepts the same\nshape on `agents tasks schedule --interval`.').meta({ omitempty: true }).optional(),
  last_ran_at: z.string().nullable().describe("RFC3339 timestamp of the most recent invocation \u2014 this row's\nnewest `tasks_runs` entry. `None` until the runner has fired\nthis version at least once (runs are tracked per-version).").meta({ omitempty: true }).optional(),
  name: z.string().describe("The `--name` passed to `agents tasks schedule`. Unique per\n`agent_instance_hierarchy`."),
  plugin: CliCommandTasksListPluginSchema.nullable().describe("The plugin that registered this schedule (its `(owner,\nrepository, version)` coordinate), or `None` when it was not\nscheduled by a plugin.").meta({ omitempty: true }).optional(),
  version: z.number().int().min(0).max(18446744073709552e3).describe("This row's version: `1` for a freshly scheduled task,\n`max + 1` for each `tasks schedule --overwrite` (each version\nis its own row; only the newest lists).")
}).describe("One schedule row.").meta({ title: "cli.command.tasks.list.ResponseItem" });

// src/viewer/command/tasks/list.ts
function tasksListExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "tasks/list" }), z.union([CliErrorSchema, CliCommandTasksListResponseItemSchema]));
}
function tasksListExecuteTransform(request, transform) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "tasks/list" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function tasksListRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "tasks/list/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tasks list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function tasksListRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "tasks/list/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tasks list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function tasksListResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "tasks/list/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tasks list response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function tasksListResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "tasks/list/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tasks list response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandTasksRunSuccessResponseItemSchema = z.object({
  agent_instance_hierarchy: z.string().describe("The source schedule's `agent_instance_hierarchy`."),
  name: z.string().describe("The source schedule's `--name`."),
  plugin: CliCommandTasksListPluginSchema.nullable().describe("The plugin that registered the source schedule, if any.").meta({ omitempty: true }).optional(),
  success: z.boolean().describe("Whether the task's stream completed without a trailing error."),
  version: z.number().int().min(0).max(18446744073709552e3).describe("The source schedule's version (`1` on first creation,\nincremented per `schedule --overwrite`).")
}).describe("One per-task completion summary (default mode): the same schedule\nidentity as [`ValueResponseItem`], with `success` in lieu of\n`value`. `success` is `false` iff the task's FINAL emitted item was\nan error (a task that emitted nothing is a success). The task's\nfull output is in `tasks_logs` regardless.").meta({ title: "cli.command.tasks.run.SuccessResponseItem" });
var CliCommandTasksRunValueResponseItemSchema = z.object({
  agent_instance_hierarchy: z.string().describe("The source schedule's `agent_instance_hierarchy`."),
  name: z.string().describe("The source schedule's `--name`."),
  plugin: CliCommandTasksListPluginSchema.nullable().describe("The plugin that registered the source schedule, if any.").meta({ omitempty: true }).optional(),
  value: JsonValueSchema.describe("What the scheduled command emitted \u2014 either the typed root item\n(no transform) or post-transform JSON. See [`RunValue`]. Schema\nis opaqued to `serde_json::Value`."),
  version: z.number().int().min(0).max(18446744073709552e3).describe("The source schedule's version (`1` on first creation,\nincremented per `schedule --overwrite`).")
}).describe("One output item from one fired schedule's in-process stream\n(`stream_all` mode). The first four fields identify the source\nschedule; `value` is the typed root\n[`crate::cli::command::ResponseItem`] emitted by the scheduled cli\nleaf \u2014 boxed because the root union transitively contains *this*\ntype (`agents \u2192 tasks \u2192 run`), and boxing is what makes the\nrecursion sized.\n\nThe `value` field's JSON schema is opaqued to `serde_json::Value`\n(renders as bare `{}` aka JsonValue) so the published schema\ndoesn't inline the entire root union \u2014 that's the TS7056 blowup\nthe root and tier aggregates dodge by being `json_schema_ignore`.\nDownstream SDKs see `value: JsonValue` on the typed `execute`\npath; consumers that want to peer inside parse it case-by-case.").meta({ title: "cli.command.tasks.run.ValueResponseItem" });

// src/cli/command/tasks/run/responseItem.ts
var CliCommandTasksRunResponseItemSchema = z.union([CliCommandTasksRunValueResponseItemSchema.meta({ "title": "cli.command.tasks.run.ValueResponseItem", "variantTitle": "Value" }), CliCommandTasksRunSuccessResponseItemSchema.meta({ "title": "cli.command.tasks.run.SuccessResponseItem", "variantTitle": "Success" })]).describe("One stream item from `tasks run`. Untagged \u2014 the variants'\nrequired fields (`value` vs `success`) are disjoint, so the wire\nshape is just the inner object. Which variant flows is decided by\nthe request's `stream_all`: `true` streams every emitted item as a\n[`ValueResponseItem`] (whose `value` is the typed root item for a\nno-transform command, or the post-transform JSON otherwise \u2014 see\n[`RunValue`]); `false` (default) yields exactly one\n[`SuccessResponseItem`] per task when its stream completes.").meta({ title: "cli.command.tasks.run.ResponseItem" });

// src/viewer/command/tasks/run.ts
function tasksRunExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "tasks/run" }), z.union([CliErrorSchema, CliCommandTasksRunResponseItemSchema]));
}
function tasksRunExecuteTransform(request, transform) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "tasks/run" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function tasksRunRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "tasks/run/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tasks run request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function tasksRunRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "tasks/run/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tasks run request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function tasksRunResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "tasks/run/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tasks run response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function tasksRunResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "tasks/run/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tasks run response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandTasksScheduleResponseSchema = z.object({
  agent_instance_hierarchy: z.string(),
  name: z.string(),
  version: z.number().int().min(0).max(18446744073709552e3)
}).describe("The created schedule's user-facing identity: its `--name`, the\ncaller hierarchy it was registered under, and the version this call\nminted (`1` on first creation, `max + 1` per `--overwrite` \u2014 each\nversion is its own row; older versions are shadowed but kept for\nper-version run history).").meta({ title: "cli.command.tasks.schedule.Response" });

// src/viewer/command/tasks/schedule.ts
async function tasksScheduleExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "tasks/schedule" }), z.union([CliErrorSchema, CliCommandTasksScheduleResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tasks schedule: cli produced no output before the end marker");
  }
  return first;
}
async function tasksScheduleExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "tasks/schedule" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tasks schedule: cli produced no output before the end marker");
  }
  return first;
}
async function tasksScheduleRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "tasks/schedule/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tasks schedule request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function tasksScheduleRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "tasks/schedule/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tasks schedule request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function tasksScheduleResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "tasks/schedule/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tasks schedule response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function tasksScheduleResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "tasks/schedule/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tasks schedule response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandToolsGetResponseManifestSchema = z.object({
  description: z.string(),
  exec: CliCommandToolsGetExecSchema,
  name: z.string(),
  owner: z.string(),
  version: z.string()
}).describe("Wire response for `tools get` \u2014 a lean projection of the on-disk\nmanifest. `exec` is required (a tool always has a command). The\non-disk-only fields (`cli_zip`, `source`) are intentionally absent;\nthe CLI owns the full on-disk shape in its own `filesystem::tools`\nmanifest types.").meta({ title: "cli.command.tools.get.ResponseManifest" });

// src/viewer/command/tools/get.ts
async function toolsGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "tools/get" }), z.union([CliErrorSchema, CliCommandToolsGetResponseManifestSchema.nullable()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools get: cli produced no output before the end marker");
  }
  return first;
}
async function toolsGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "tools/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools get: cli produced no output before the end marker");
  }
  return first;
}
async function toolsGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "tools/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function toolsGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "tools/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function toolsGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "tools/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function toolsGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "tools/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandToolsInstallFilesystemResponseSchema = z.object({
  instructions: z.string()
}).meta({ title: "cli.command.tools.install.filesystem.Response" });

// src/viewer/command/tools/install/filesystem.ts
async function toolsInstallFilesystemExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "tools/install/filesystem" }), z.union([CliErrorSchema, CliCommandToolsInstallFilesystemResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools install filesystem: cli produced no output before the end marker");
  }
  return first;
}
async function toolsInstallFilesystemExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "tools/install/filesystem" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools install filesystem: cli produced no output before the end marker");
  }
  return first;
}
async function toolsInstallFilesystemRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "tools/install/filesystem/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools install filesystem request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function toolsInstallFilesystemRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "tools/install/filesystem/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools install filesystem request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function toolsInstallFilesystemResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "tools/install/filesystem/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools install filesystem response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function toolsInstallFilesystemResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "tools/install/filesystem/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools install filesystem response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandToolsInstallGithubResponseSchema = z.object({
  installed: z.boolean()
}).meta({ title: "cli.command.tools.install.github.Response" });

// src/viewer/command/tools/install/github.ts
async function toolsInstallGithubExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "tools/install/github" }), z.union([CliErrorSchema, CliCommandToolsInstallGithubResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools install github: cli produced no output before the end marker");
  }
  return first;
}
async function toolsInstallGithubExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "tools/install/github" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools install github: cli produced no output before the end marker");
  }
  return first;
}
async function toolsInstallGithubRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "tools/install/github/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools install github request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function toolsInstallGithubRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "tools/install/github/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools install github request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function toolsInstallGithubResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "tools/install/github/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools install github response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function toolsInstallGithubResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "tools/install/github/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools install github response_schema: cli produced no output before the end marker");
  }
  return first;
}
function toolsListExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "tools/list" }), z.union([CliErrorSchema, CliCommandToolsGetResponseManifestSchema]));
}
function toolsListExecuteTransform(request, transform) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "tools/list" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function toolsListRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "tools/list/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function toolsListRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "tools/list/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function toolsListResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "tools/list/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools list response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function toolsListResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "tools/list/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools list response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandToolsRunResponseItemSchema = z.union([z.string().meta({ "variantTitle": "Stdout" }), CliErrorSchema.meta({ "title": "cli.Error", "variantTitle": "Stderr" })]).meta({ title: "cli.command.tools.run.ResponseItem" });

// src/viewer/command/tools/run.ts
function toolsRunExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "tools/run" }), z.union([CliErrorSchema, CliCommandToolsRunResponseItemSchema]));
}
function toolsRunExecuteTransform(request, transform) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "tools/run" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function toolsRunRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "tools/run/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools run request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function toolsRunRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "tools/run/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools run request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function toolsRunResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "tools/run/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools run response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function toolsRunResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "tools/run/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools run response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandUpdateResponseSkipReasonSchema = z.enum(["dev_tree", "unsupported_platform", "incomplete_release"]).meta({ title: "cli.command.update.ResponseSkipReason" });

// src/cli/command/update/responseItem.ts
var CliCommandUpdateResponseItemSchema = z.union([z.object({
  asset_name: z.string(),
  current_version: z.string(),
  type: z.literal("checking")
}).meta({ "variantTitle": "Checking" }), z.object({
  asset_name: z.string(),
  current_version: z.string(),
  remote_version: z.string(),
  type: z.literal("found"),
  url: z.string()
}).meta({ "variantTitle": "Found" }), z.object({
  current_version: z.string(),
  remote_version: z.string(),
  type: z.literal("installed")
}).meta({ "variantTitle": "Installed" }), z.object({
  reason: CliCommandUpdateResponseSkipReasonSchema,
  type: z.literal("skipped")
}).meta({ "variantTitle": "Skipped" }), z.object({
  current_version: z.string(),
  remote_version: z.string(),
  type: z.literal("up_to_date")
}).meta({ "variantTitle": "UpToDate" })]).describe("Per-stage update event. Update is a streaming leaf \u2014 one\n`ResponseItem` per (asset, stage) pair as the update progresses\nthrough the four shipped binaries (cli, api, viewer, mcp).").meta({ title: "cli.command.update.ResponseItem" });

// src/viewer/command/update.ts
function updateExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "update" }), z.union([CliErrorSchema, CliCommandUpdateResponseItemSchema]));
}
function updateExecuteTransform(request, transform) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "update" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function updateRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "update/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("update request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function updateRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "update/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("update request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function updateResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "update/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("update response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function updateResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "update/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("update response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandViewerConfigAddressGetResponseSchema = z.object({
  address: z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.viewer.config.address.get.Response" });

// src/viewer/command/viewer/config/address/get.ts
async function viewerConfigAddressGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/config/address/get" }), z.union([CliErrorSchema, CliCommandViewerConfigAddressGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config address get: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigAddressGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/config/address/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config address get: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigAddressGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/config/address/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config address get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigAddressGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/config/address/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config address get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigAddressGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/config/address/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config address get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigAddressGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/config/address/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config address get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigAddressSetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "viewer/config/address/set" }), z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config address set: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigAddressSetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "viewer/config/address/set" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config address set: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigAddressSetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/config/address/set/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config address set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigAddressSetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/config/address/set/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config address set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigAddressSetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/config/address/set/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config address set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigAddressSetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/config/address/set/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config address set response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandViewerConfigGetResponseSchema = z.object({
  address: z.string().nullable().meta({ omitempty: true }).optional(),
  secret: z.string().nullable().meta({ omitempty: true }).optional(),
  signature: z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.viewer.config.get.Response" });

// src/viewer/command/viewer/config/get.ts
async function viewerConfigGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/config/get" }), z.union([CliErrorSchema, CliCommandViewerConfigGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config get: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/config/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config get: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/config/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/config/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/config/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/config/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandViewerConfigSecretGetResponseSchema = z.object({
  secret: z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.viewer.config.secret.get.Response" });

// src/viewer/command/viewer/config/secret/get.ts
async function viewerConfigSecretGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/config/secret/get" }), z.union([CliErrorSchema, CliCommandViewerConfigSecretGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config secret get: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigSecretGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/config/secret/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config secret get: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigSecretGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/config/secret/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config secret get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigSecretGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/config/secret/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config secret get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigSecretGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/config/secret/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config secret get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigSecretGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/config/secret/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config secret get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigSecretSetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "viewer/config/secret/set" }), z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config secret set: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigSecretSetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "viewer/config/secret/set" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config secret set: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigSecretSetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/config/secret/set/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config secret set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigSecretSetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/config/secret/set/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config secret set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigSecretSetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/config/secret/set/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config secret set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigSecretSetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/config/secret/set/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config secret set response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandViewerConfigSignatureGetResponseSchema = z.object({
  signature: z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.viewer.config.signature.get.Response" });

// src/viewer/command/viewer/config/signature/get.ts
async function viewerConfigSignatureGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/config/signature/get" }), z.union([CliErrorSchema, CliCommandViewerConfigSignatureGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config signature get: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigSignatureGetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/config/signature/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config signature get: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigSignatureGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/config/signature/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config signature get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigSignatureGetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/config/signature/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config signature get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigSignatureGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/config/signature/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config signature get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigSignatureGetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/config/signature/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config signature get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigSignatureSetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "viewer/config/signature/set" }), z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config signature set: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigSignatureSetExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "viewer/config/signature/set" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config signature set: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigSignatureSetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/config/signature/set/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config signature set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigSignatureSetRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/config/signature/set/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config signature set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigSignatureSetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/config/signature/set/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config signature set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerConfigSignatureSetResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/config/signature/set/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer config signature set response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandViewerGenerateSecretSignaturePairResponseSchema = z.object({
  secret: z.string(),
  signature: z.string()
}).meta({ title: "cli.command.viewer.generate_secret_signature_pair.Response" });

// src/viewer/command/viewer/generate_secret_signature_pair.ts
async function viewerGenerateSecretSignaturePairExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/generate_secret_signature_pair" }), z.union([CliErrorSchema, CliCommandViewerGenerateSecretSignaturePairResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer generate_secret_signature_pair: cli produced no output before the end marker");
  }
  return first;
}
async function viewerGenerateSecretSignaturePairExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/generate_secret_signature_pair" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer generate_secret_signature_pair: cli produced no output before the end marker");
  }
  return first;
}
async function viewerGenerateSecretSignaturePairRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/generate_secret_signature_pair/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer generate_secret_signature_pair request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerGenerateSecretSignaturePairRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/generate_secret_signature_pair/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer generate_secret_signature_pair request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerGenerateSecretSignaturePairResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/generate_secret_signature_pair/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer generate_secret_signature_pair response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerGenerateSecretSignaturePairResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/generate_secret_signature_pair/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer generate_secret_signature_pair response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandViewerKillResponseSchema = z.object({
  killed: z.number().int().min(0).max(4294967295)
}).meta({ title: "cli.command.viewer.kill.Response" });

// src/viewer/command/viewer/kill.ts
async function viewerKillExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/kill" }), z.union([CliErrorSchema, CliCommandViewerKillResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer kill: cli produced no output before the end marker");
  }
  return first;
}
async function viewerKillExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/kill" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer kill: cli produced no output before the end marker");
  }
  return first;
}
async function viewerKillRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/kill/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer kill request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerKillRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/kill/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer kill request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerKillResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/kill/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer kill response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerKillResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/kill/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer kill response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandViewerSendResponseSchema = z.object({
  body: JsonValueSchema,
  status: z.number().int().min(0).max(65535)
}).meta({ title: "cli.command.viewer.send.Response" });

// src/viewer/command/viewer/send.ts
async function viewerSendExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/send" }), z.union([CliErrorSchema, CliCommandViewerSendResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer send: cli produced no output before the end marker");
  }
  return first;
}
async function viewerSendExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/send" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer send: cli produced no output before the end marker");
  }
  return first;
}
async function viewerSendRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/send/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer send request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerSendRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/send/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer send request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerSendResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/send/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer send response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerSendResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/send/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer send response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandViewerSpawnResponseSchema = z.object({
  listening: z.string()
}).meta({ title: "cli.command.viewer.spawn.Response" });

// src/viewer/command/viewer/spawn.ts
async function viewerSpawnExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/spawn" }), z.union([CliErrorSchema, CliCommandViewerSpawnResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer spawn: cli produced no output before the end marker");
  }
  return first;
}
async function viewerSpawnExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/spawn" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer spawn: cli produced no output before the end marker");
  }
  return first;
}
async function viewerSpawnRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/spawn/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer spawn request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerSpawnRequestSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/spawn/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer spawn request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerSpawnResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, path_type: "viewer/spawn/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer spawn response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerSpawnResponseSchemaExecuteTransform(request, transform) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, python: void 0, ...transform, path_type: "viewer/spawn/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
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
function listen(sub_type, handler) {
  if (!isInIframe()) {
    let unlisten = null;
    let cancelled = false;
    void (async () => {
      try {
        const mod = await import('./event-MA6ITXCE.js');
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

export { AgentClaudeAgentSdkAgentBaseSchema, AgentClaudeAgentSdkEffortSchema, AgentClaudeAgentSdkOutputModeSchema, AgentClaudeAgentSdkUpstreamSchema, AgentClientObjectiveaiMcpEntrySchema, AgentClientObjectiveaiMcpPluginEntrySchema, AgentClientObjectiveaiMcpPluginMcpServerSchema, AgentClientObjectiveaiMcpSchema, AgentCodexSdkAgentBaseSchema, AgentCodexSdkEffortSchema, AgentCodexSdkOutputModeSchema, AgentCodexSdkUpstreamSchema, AgentCompletionsMessageAssistantMessageExpressionSchema, AgentCompletionsMessageAssistantMessageSchema, AgentCompletionsMessageAssistantToolCallDeltaSchema, AgentCompletionsMessageAssistantToolCallExpressionSchema, AgentCompletionsMessageAssistantToolCallFunctionDeltaSchema, AgentCompletionsMessageAssistantToolCallFunctionExpressionSchema, AgentCompletionsMessageAssistantToolCallFunctionSchema, AgentCompletionsMessageAssistantToolCallSchema, AgentCompletionsMessageAssistantToolCallTypeSchema, AgentCompletionsMessageDeveloperMessageExpressionSchema, AgentCompletionsMessageDeveloperMessageSchema, AgentCompletionsMessageFileSchema, AgentCompletionsMessageImageUrlDetailSchema, AgentCompletionsMessageImageUrlSchema, AgentCompletionsMessageInputAudioSchema, AgentCompletionsMessageMessageExpressionSchema, AgentCompletionsMessageMessageSchema, AgentCompletionsMessageRichContentExpressionSchema, AgentCompletionsMessageRichContentPartExpressionSchema, AgentCompletionsMessageRichContentPartSchema, AgentCompletionsMessageRichContentSchema, AgentCompletionsMessageSimpleContentExpressionSchema, AgentCompletionsMessageSimpleContentPartExpressionSchema, AgentCompletionsMessageSimpleContentPartSchema, AgentCompletionsMessageSimpleContentSchema, AgentCompletionsMessageSystemMessageExpressionSchema, AgentCompletionsMessageSystemMessageSchema, AgentCompletionsMessageToolMessageExpressionSchema, AgentCompletionsMessageToolMessageSchema, AgentCompletionsMessageToolResponseMetadataSchema, AgentCompletionsMessageUserMessageExpressionSchema, AgentCompletionsMessageUserMessageSchema, AgentCompletionsMessageVideoUrlSchema, AgentCompletionsRequestAgentCompletionCreateParamsSchema, AgentCompletionsRequestProviderDataCollectionSchema, AgentCompletionsRequestProviderMaxPriceSchema, AgentCompletionsRequestProviderSchema, AgentCompletionsRequestProviderSortSchema, AgentCompletionsRequestResponseFormatParamSchema, AgentCompletionsRequestResponseFormatSchema, AgentCompletionsResponseAssistantRoleSchema, AgentCompletionsResponseCompletionTokensDetailsSchema, AgentCompletionsResponseCostDetailsSchema, AgentCompletionsResponseFinishReasonSchema, AgentCompletionsResponseLogprobSchema, AgentCompletionsResponseLogprobsSchema, AgentCompletionsResponsePromptTokensDetailsSchema, AgentCompletionsResponseStreamingAgentCompletionChunkSchema, AgentCompletionsResponseStreamingAssistantResponseChunkSchema, AgentCompletionsResponseStreamingMessageChunkSchema, AgentCompletionsResponseStreamingObjectSchema, AgentCompletionsResponseToolResponseSchema, AgentCompletionsResponseToolRoleSchema, AgentCompletionsResponseTopLogprobSchema, AgentCompletionsResponseUpstreamUsageSchema, AgentCompletionsResponseUsageSchema, AgentInlineAgentBaseSchema, AgentInlineAgentBaseWithFallbacksOrRemoteCommitOptionalSchema, AgentInlineAgentBaseWithFallbacksOrRemoteSchema, AgentInlineAgentBaseWithFallbacksOrRemoteWithCountSchema, AgentInlineAgentBaseWithFallbacksSchema, AgentMcpServerSchema, AgentMockAgentBaseSchema, AgentMockCallSchema, AgentMockCallToolCallSchema, AgentMockOutputModeSchema, AgentMockUpstreamSchema, AgentOpenrouterAgentBaseSchema, AgentOpenrouterContextCompressionSchema, AgentOpenrouterOutputModeSchema, AgentOpenrouterProviderQuantizationSchema, AgentOpenrouterProviderSchema, AgentOpenrouterReasoningEffortSchema, AgentOpenrouterReasoningSchema, AgentOpenrouterReasoningSummaryVerbositySchema, AgentOpenrouterStopSchema, AgentOpenrouterUpstreamSchema, AgentOpenrouterVerbositySchema, AgentUpstreamSchema, CliCommandAgentsEnqueueResponseSchema, CliCommandAgentsGetResponseSchema, CliCommandAgentsInstancesListResponseItemSchema, CliCommandAgentsLogsReadAllAssistantResponsePartSchema, CliCommandAgentsLogsReadAllClientNotificationPartSchema, CliCommandAgentsLogsReadAllClientNotificationPartTypeSchema, CliCommandAgentsLogsReadAllResponseItemSchema, CliCommandAgentsLogsReadAllToolResponsePartSchema, CliCommandAgentsLogsReadAllToolResponsePartTypeSchema, CliCommandAgentsLogsReadIdResponseSchema, CliCommandAgentsLogsReadSubscribeAgentsInactiveTagSchema, CliCommandAgentsLogsReadSubscribeResponseItemSchema, CliCommandAgentsMessageResponseSchema, CliCommandAgentsPublishResponseSchema, CliCommandAgentsQueueDeleteResponseSchema, CliCommandAgentsQueueDeliverAgentActiveResponseItemSchema, CliCommandAgentsQueueDeliverAgentActiveTypeSchema, CliCommandAgentsQueueDeliverAgentSpawnedResponseItemSchema, CliCommandAgentsQueueDeliverAgentSpawnedTypeSchema, CliCommandAgentsQueueDeliverAllAgentsActiveSchema, CliCommandAgentsQueueDeliverResponseItemSchema, CliCommandAgentsQueueDeliverTagActiveResponseItemSchema, CliCommandAgentsQueueDeliverTagActiveTypeSchema, CliCommandAgentsQueueDeliverTagSpawnedResponseItemSchema, CliCommandAgentsQueueDeliverTagSpawnedTypeSchema, CliCommandAgentsQueueDeliverValueResponseItemSchema, CliCommandAgentsQueueReadPendingQueuePartSchema, CliCommandAgentsQueueReadPendingResponseItemSchema, CliCommandAgentsSpawnResponseItemSchema, CliCommandAgentsTagsApplyResponseSchema, CliCommandApiConfigAddressGetResponseSchema, CliCommandApiConfigCommitAuthorEmailGetResponseSchema, CliCommandApiConfigCommitAuthorNameGetResponseSchema, CliCommandApiConfigGetResponseSchema, CliCommandApiConfigGithubAuthorizationGetResponseSchema, CliCommandApiConfigHttpRefererGetResponseSchema, CliCommandApiConfigMcpAuthorizationGetResponseSchema, CliCommandApiConfigObjectiveaiAuthorizationGetResponseSchema, CliCommandApiConfigOpenrouterAuthorizationGetResponseSchema, CliCommandApiConfigUserAgentGetResponseSchema, CliCommandApiConfigXTitleGetResponseSchema, CliCommandApiKillResponseSchema, CliCommandApiSpawnResponseSchema, CliCommandDbConfigAddressGetResponseSchema, CliCommandDbConfigDatabaseGetResponseSchema, CliCommandDbConfigGetResponseSchema, CliCommandDbConfigPasswordGetResponseSchema, CliCommandDbConfigUserGetResponseSchema, CliCommandDbKillResponseSchema, CliCommandDbQueryColumnSchema, CliCommandDbQueryResponseSchema, CliCommandDbSpawnResponseSchema, CliCommandFunctionsExecuteStandardResponseItemSchema, CliCommandFunctionsExecuteSwissSystemResponseItemSchema, CliCommandFunctionsGetResponseSchema, CliCommandFunctionsProfilesGetResponseSchema, CliCommandFunctionsProfilesPublishResponseSchema, CliCommandFunctionsPublishResponseSchema, CliCommandMcpConfigAddressGetResponseSchema, CliCommandMcpConfigGetResponseSchema, CliCommandMcpConfigPortGetResponseSchema, CliCommandMcpKillResponseSchema, CliCommandMcpSpawnResponseSchema, CliCommandOkSchema, CliCommandPluginsGetResponseHttpMethodSchema, CliCommandPluginsGetResponseManifestSchema, CliCommandPluginsGetResponseMcpServerSchema, CliCommandPluginsGetResponseViewerRouteSchema, CliCommandPluginsInstallFilesystemResponseSchema, CliCommandPluginsInstallGithubResponseSchema, CliCommandPluginsRunMcpSchema, CliCommandPluginsRunMcpTypeSchema, CliCommandPluginsRunResponseItemSchema, CliCommandSwarmsGetResponseSchema, CliCommandSwarmsPublishResponseSchema, CliCommandTasksListPluginSchema, CliCommandTasksListResponseItemSchema, CliCommandTasksRunResponseItemSchema, CliCommandTasksRunSuccessResponseItemSchema, CliCommandTasksRunValueResponseItemSchema, CliCommandTasksScheduleResponseSchema, CliCommandToolsGetExecSchema, CliCommandToolsGetResponseManifestSchema, CliCommandToolsInstallFilesystemResponseSchema, CliCommandToolsInstallGithubResponseSchema, CliCommandToolsRunResponseItemSchema, CliCommandUpdateResponseItemSchema, CliCommandUpdateResponseSkipReasonSchema, CliCommandViewerConfigAddressGetResponseSchema, CliCommandViewerConfigGetResponseSchema, CliCommandViewerConfigSecretGetResponseSchema, CliCommandViewerConfigSignatureGetResponseSchema, CliCommandViewerGenerateSecretSignaturePairResponseSchema, CliCommandViewerKillResponseSchema, CliCommandViewerSendResponseSchema, CliCommandViewerSpawnResponseSchema, CliErrorSchema, CliErrorTypeSchema, CliLevelSchema, CliStream, ErrorResponseErrorSchema, FunctionsAlphaInlineFunctionSchema, FunctionsAlphaScalarBranchTaskExpressionSchema, FunctionsAlphaScalarInlineFunctionSchema, FunctionsAlphaScalarLeafTaskExpressionSchema, FunctionsAlphaScalarPlaceholderScalarFunctionTaskExpressionSchema, FunctionsAlphaScalarScalarFunctionTaskExpressionSchema, FunctionsAlphaScalarVectorCompletionTaskExpressionSchema, FunctionsAlphaVectorBranchTaskExpressionSchema, FunctionsAlphaVectorExpressionVectorFunctionInputSchemaSchema, FunctionsAlphaVectorExpressionVectorFunctionInputValueExpressionSchema, FunctionsAlphaVectorInlineFunctionSchema, FunctionsAlphaVectorLeafTaskExpressionSchema, FunctionsAlphaVectorPlaceholderScalarFunctionTaskExpressionSchema, FunctionsAlphaVectorPlaceholderVectorFunctionTaskExpressionSchema, FunctionsAlphaVectorScalarFunctionTaskExpressionSchema, FunctionsAlphaVectorVectorCompletionTaskExpressionSchema, FunctionsAlphaVectorVectorFunctionTaskExpressionSchema, FunctionsExecutionsRequestFunctionExecutionCreateParamsSchema, FunctionsExecutionsRequestReasoningSchema, FunctionsExecutionsRequestStrategySchema, FunctionsExecutionsResponseOutputSchema, FunctionsExecutionsResponseStreamingFunctionExecutionChunkSchema, FunctionsExecutionsResponseStreamingFunctionExecutionTaskChunkSchema, FunctionsExecutionsResponseStreamingObjectSchema, FunctionsExecutionsResponseStreamingReasoningSummaryChunkSchema, FunctionsExecutionsResponseStreamingTaskChunkSchema, FunctionsExecutionsResponseStreamingVectorCompletionTaskChunkSchema, FunctionsExpressionAnyOfInputSchemaSchema, FunctionsExpressionArrayInputSchemaSchema, FunctionsExpressionArrayInputSchemaTypeSchema, FunctionsExpressionAudioInputSchemaSchema, FunctionsExpressionAudioInputSchemaTypeSchema, FunctionsExpressionBooleanInputSchemaSchema, FunctionsExpressionBooleanInputSchemaTypeSchema, FunctionsExpressionExpressionSchema, FunctionsExpressionFileInputSchemaSchema, FunctionsExpressionFileInputSchemaTypeSchema, FunctionsExpressionImageInputSchemaSchema, FunctionsExpressionImageInputSchemaTypeSchema, FunctionsExpressionInputSchemaSchema, FunctionsExpressionInputValueExpressionSchema, FunctionsExpressionInputValueSchema, FunctionsExpressionIntegerInputSchemaSchema, FunctionsExpressionIntegerInputSchemaTypeSchema, FunctionsExpressionNumberInputSchemaSchema, FunctionsExpressionNumberInputSchemaTypeSchema, FunctionsExpressionObjectInputSchemaSchema, FunctionsExpressionObjectInputSchemaTypeSchema, FunctionsExpressionSpecialSchema, FunctionsExpressionStringInputSchemaSchema, FunctionsExpressionStringInputSchemaTypeSchema, FunctionsExpressionTaskOutputSchema, FunctionsExpressionVideoInputSchemaSchema, FunctionsExpressionVideoInputSchemaTypeSchema, FunctionsFullInlineFunctionOrRemoteCommitOptionalSchema, FunctionsFullInlineFunctionSchema, FunctionsInlineFunctionSchema, FunctionsInlineProfileOrRemoteCommitOptionalSchema, FunctionsInlineProfileSchema, FunctionsInlineTasksProfileSchema, FunctionsPlaceholderScalarFunctionTaskExpressionSchema, FunctionsPlaceholderVectorFunctionTaskExpressionSchema, FunctionsScalarFunctionTaskExpressionSchema, FunctionsTaskExpressionSchema, FunctionsTaskProfileSchema, FunctionsVectorCompletionTaskExpressionSchema, FunctionsVectorFunctionTaskExpressionSchema, JsonValueSchema, RemotePathCommitOptionalSchema, RemotePathSchema, SwarmInlineSwarmBaseOrRemoteCommitOptionalSchema, SwarmInlineSwarmBaseSchema, VectorCompletionsRequestVectorCompletionCreateParamsSchema, VectorCompletionsResponseStreamingAgentCompletionChunkSchema, VectorCompletionsResponseStreamingObjectSchema, VectorCompletionsResponseVoteSchema, ViewerEventSchema, WeightsEntrySchema, WeightsSchema, __resetForTests, agentsEnqueueExecute, agentsEnqueueExecuteTransform, agentsEnqueueRequestSchemaExecute, agentsEnqueueRequestSchemaExecuteTransform, agentsEnqueueResponseSchemaExecute, agentsEnqueueResponseSchemaExecuteTransform, agentsGetExecute, agentsGetExecuteTransform, agentsGetRequestSchemaExecute, agentsGetRequestSchemaExecuteTransform, agentsGetResponseSchemaExecute, agentsGetResponseSchemaExecuteTransform, agentsInstancesGetExecute, agentsInstancesGetExecuteTransform, agentsInstancesGetRequestSchemaExecute, agentsInstancesGetRequestSchemaExecuteTransform, agentsInstancesGetResponseSchemaExecute, agentsInstancesGetResponseSchemaExecuteTransform, agentsInstancesListExecute, agentsInstancesListExecuteTransform, agentsInstancesListRequestSchemaExecute, agentsInstancesListRequestSchemaExecuteTransform, agentsInstancesListResponseSchemaExecute, agentsInstancesListResponseSchemaExecuteTransform, agentsListExecute, agentsListExecuteTransform, agentsListRequestSchemaExecute, agentsListRequestSchemaExecuteTransform, agentsListResponseSchemaExecute, agentsListResponseSchemaExecuteTransform, agentsLogsReadAllExecute, agentsLogsReadAllExecuteTransform, agentsLogsReadAllRequestSchemaExecute, agentsLogsReadAllRequestSchemaExecuteTransform, agentsLogsReadAllResponseSchemaExecute, agentsLogsReadAllResponseSchemaExecuteTransform, agentsLogsReadIdExecute, agentsLogsReadIdExecuteTransform, agentsLogsReadIdRequestSchemaExecute, agentsLogsReadIdRequestSchemaExecuteTransform, agentsLogsReadIdResponseSchemaExecute, agentsLogsReadIdResponseSchemaExecuteTransform, agentsLogsReadPendingExecute, agentsLogsReadPendingExecuteTransform, agentsLogsReadPendingRequestSchemaExecute, agentsLogsReadPendingRequestSchemaExecuteTransform, agentsLogsReadPendingResponseSchemaExecute, agentsLogsReadPendingResponseSchemaExecuteTransform, agentsLogsReadSubscribeExecute, agentsLogsReadSubscribeExecuteTransform, agentsLogsReadSubscribeRequestSchemaExecute, agentsLogsReadSubscribeRequestSchemaExecuteTransform, agentsLogsReadSubscribeResponseSchemaExecute, agentsLogsReadSubscribeResponseSchemaExecuteTransform, agentsMessageExecute, agentsMessageExecuteTransform, agentsMessageRequestSchemaExecute, agentsMessageRequestSchemaExecuteTransform, agentsMessageResponseSchemaExecute, agentsMessageResponseSchemaExecuteTransform, agentsPublishExecute, agentsPublishExecuteTransform, agentsPublishRequestSchemaExecute, agentsPublishRequestSchemaExecuteTransform, agentsPublishResponseSchemaExecute, agentsPublishResponseSchemaExecuteTransform, agentsQueueDeleteExecute, agentsQueueDeleteExecuteTransform, agentsQueueDeleteRequestSchemaExecute, agentsQueueDeleteRequestSchemaExecuteTransform, agentsQueueDeleteResponseSchemaExecute, agentsQueueDeleteResponseSchemaExecuteTransform, agentsQueueDeliverExecute, agentsQueueDeliverExecuteTransform, agentsQueueDeliverRequestSchemaExecute, agentsQueueDeliverRequestSchemaExecuteTransform, agentsQueueDeliverResponseSchemaExecute, agentsQueueDeliverResponseSchemaExecuteTransform, agentsQueueReadIdExecute, agentsQueueReadIdExecuteTransform, agentsQueueReadIdRequestSchemaExecute, agentsQueueReadIdRequestSchemaExecuteTransform, agentsQueueReadIdResponseSchemaExecute, agentsQueueReadIdResponseSchemaExecuteTransform, agentsQueueReadPendingExecute, agentsQueueReadPendingExecuteTransform, agentsQueueReadPendingRequestSchemaExecute, agentsQueueReadPendingRequestSchemaExecuteTransform, agentsQueueReadPendingResponseSchemaExecute, agentsQueueReadPendingResponseSchemaExecuteTransform, agentsSpawnExecute, agentsSpawnExecuteStreaming, agentsSpawnExecuteStreamingTransform, agentsSpawnExecuteTransform, agentsSpawnRequestSchemaExecute, agentsSpawnRequestSchemaExecuteTransform, agentsSpawnResponseSchemaExecute, agentsSpawnResponseSchemaExecuteTransform, agentsTagsApplyExecute, agentsTagsApplyExecuteTransform, agentsTagsApplyRequestSchemaExecute, agentsTagsApplyRequestSchemaExecuteTransform, agentsTagsApplyResponseSchemaExecute, agentsTagsApplyResponseSchemaExecuteTransform, agentsTagsLookupRequestSchemaExecute, agentsTagsLookupRequestSchemaExecuteTransform, agentsTagsLookupResponseSchemaExecute, agentsTagsLookupResponseSchemaExecuteTransform, agentsWaitExecute, agentsWaitExecuteTransform, agentsWaitRequestSchemaExecute, agentsWaitRequestSchemaExecuteTransform, agentsWaitResponseSchemaExecute, agentsWaitResponseSchemaExecuteTransform, apiConfigAddressGetExecute, apiConfigAddressGetExecuteTransform, apiConfigAddressGetRequestSchemaExecute, apiConfigAddressGetRequestSchemaExecuteTransform, apiConfigAddressGetResponseSchemaExecute, apiConfigAddressGetResponseSchemaExecuteTransform, apiConfigAddressSetExecute, apiConfigAddressSetExecuteTransform, apiConfigAddressSetRequestSchemaExecute, apiConfigAddressSetRequestSchemaExecuteTransform, apiConfigAddressSetResponseSchemaExecute, apiConfigAddressSetResponseSchemaExecuteTransform, apiConfigCommitAuthorEmailGetExecute, apiConfigCommitAuthorEmailGetExecuteTransform, apiConfigCommitAuthorEmailGetRequestSchemaExecute, apiConfigCommitAuthorEmailGetRequestSchemaExecuteTransform, apiConfigCommitAuthorEmailGetResponseSchemaExecute, apiConfigCommitAuthorEmailGetResponseSchemaExecuteTransform, apiConfigCommitAuthorEmailSetExecute, apiConfigCommitAuthorEmailSetExecuteTransform, apiConfigCommitAuthorEmailSetRequestSchemaExecute, apiConfigCommitAuthorEmailSetRequestSchemaExecuteTransform, apiConfigCommitAuthorEmailSetResponseSchemaExecute, apiConfigCommitAuthorEmailSetResponseSchemaExecuteTransform, apiConfigCommitAuthorNameGetExecute, apiConfigCommitAuthorNameGetExecuteTransform, apiConfigCommitAuthorNameGetRequestSchemaExecute, apiConfigCommitAuthorNameGetRequestSchemaExecuteTransform, apiConfigCommitAuthorNameGetResponseSchemaExecute, apiConfigCommitAuthorNameGetResponseSchemaExecuteTransform, apiConfigCommitAuthorNameSetExecute, apiConfigCommitAuthorNameSetExecuteTransform, apiConfigCommitAuthorNameSetRequestSchemaExecute, apiConfigCommitAuthorNameSetRequestSchemaExecuteTransform, apiConfigCommitAuthorNameSetResponseSchemaExecute, apiConfigCommitAuthorNameSetResponseSchemaExecuteTransform, apiConfigGetExecute, apiConfigGetExecuteTransform, apiConfigGetRequestSchemaExecute, apiConfigGetRequestSchemaExecuteTransform, apiConfigGetResponseSchemaExecute, apiConfigGetResponseSchemaExecuteTransform, apiConfigGithubAuthorizationGetExecute, apiConfigGithubAuthorizationGetExecuteTransform, apiConfigGithubAuthorizationGetRequestSchemaExecute, apiConfigGithubAuthorizationGetRequestSchemaExecuteTransform, apiConfigGithubAuthorizationGetResponseSchemaExecute, apiConfigGithubAuthorizationGetResponseSchemaExecuteTransform, apiConfigGithubAuthorizationSetExecute, apiConfigGithubAuthorizationSetExecuteTransform, apiConfigGithubAuthorizationSetRequestSchemaExecute, apiConfigGithubAuthorizationSetRequestSchemaExecuteTransform, apiConfigGithubAuthorizationSetResponseSchemaExecute, apiConfigGithubAuthorizationSetResponseSchemaExecuteTransform, apiConfigHttpRefererGetExecute, apiConfigHttpRefererGetExecuteTransform, apiConfigHttpRefererGetRequestSchemaExecute, apiConfigHttpRefererGetRequestSchemaExecuteTransform, apiConfigHttpRefererGetResponseSchemaExecute, apiConfigHttpRefererGetResponseSchemaExecuteTransform, apiConfigHttpRefererSetExecute, apiConfigHttpRefererSetExecuteTransform, apiConfigHttpRefererSetRequestSchemaExecute, apiConfigHttpRefererSetRequestSchemaExecuteTransform, apiConfigHttpRefererSetResponseSchemaExecute, apiConfigHttpRefererSetResponseSchemaExecuteTransform, apiConfigMcpAuthorizationAddExecute, apiConfigMcpAuthorizationAddExecuteTransform, apiConfigMcpAuthorizationAddRequestSchemaExecute, apiConfigMcpAuthorizationAddRequestSchemaExecuteTransform, apiConfigMcpAuthorizationAddResponseSchemaExecute, apiConfigMcpAuthorizationAddResponseSchemaExecuteTransform, apiConfigMcpAuthorizationDelExecute, apiConfigMcpAuthorizationDelExecuteTransform, apiConfigMcpAuthorizationDelRequestSchemaExecute, apiConfigMcpAuthorizationDelRequestSchemaExecuteTransform, apiConfigMcpAuthorizationDelResponseSchemaExecute, apiConfigMcpAuthorizationDelResponseSchemaExecuteTransform, apiConfigMcpAuthorizationGetExecute, apiConfigMcpAuthorizationGetExecuteTransform, apiConfigMcpAuthorizationGetRequestSchemaExecute, apiConfigMcpAuthorizationGetRequestSchemaExecuteTransform, apiConfigMcpAuthorizationGetResponseSchemaExecute, apiConfigMcpAuthorizationGetResponseSchemaExecuteTransform, apiConfigObjectiveaiAuthorizationGetExecute, apiConfigObjectiveaiAuthorizationGetExecuteTransform, apiConfigObjectiveaiAuthorizationGetRequestSchemaExecute, apiConfigObjectiveaiAuthorizationGetRequestSchemaExecuteTransform, apiConfigObjectiveaiAuthorizationGetResponseSchemaExecute, apiConfigObjectiveaiAuthorizationGetResponseSchemaExecuteTransform, apiConfigObjectiveaiAuthorizationSetExecute, apiConfigObjectiveaiAuthorizationSetExecuteTransform, apiConfigObjectiveaiAuthorizationSetRequestSchemaExecute, apiConfigObjectiveaiAuthorizationSetRequestSchemaExecuteTransform, apiConfigObjectiveaiAuthorizationSetResponseSchemaExecute, apiConfigObjectiveaiAuthorizationSetResponseSchemaExecuteTransform, apiConfigOpenrouterAuthorizationGetExecute, apiConfigOpenrouterAuthorizationGetExecuteTransform, apiConfigOpenrouterAuthorizationGetRequestSchemaExecute, apiConfigOpenrouterAuthorizationGetRequestSchemaExecuteTransform, apiConfigOpenrouterAuthorizationGetResponseSchemaExecute, apiConfigOpenrouterAuthorizationGetResponseSchemaExecuteTransform, apiConfigOpenrouterAuthorizationSetExecute, apiConfigOpenrouterAuthorizationSetExecuteTransform, apiConfigOpenrouterAuthorizationSetRequestSchemaExecute, apiConfigOpenrouterAuthorizationSetRequestSchemaExecuteTransform, apiConfigOpenrouterAuthorizationSetResponseSchemaExecute, apiConfigOpenrouterAuthorizationSetResponseSchemaExecuteTransform, apiConfigUserAgentGetExecute, apiConfigUserAgentGetExecuteTransform, apiConfigUserAgentGetRequestSchemaExecute, apiConfigUserAgentGetRequestSchemaExecuteTransform, apiConfigUserAgentGetResponseSchemaExecute, apiConfigUserAgentGetResponseSchemaExecuteTransform, apiConfigUserAgentSetExecute, apiConfigUserAgentSetExecuteTransform, apiConfigUserAgentSetRequestSchemaExecute, apiConfigUserAgentSetRequestSchemaExecuteTransform, apiConfigUserAgentSetResponseSchemaExecute, apiConfigUserAgentSetResponseSchemaExecuteTransform, apiConfigXTitleGetExecute, apiConfigXTitleGetExecuteTransform, apiConfigXTitleGetRequestSchemaExecute, apiConfigXTitleGetRequestSchemaExecuteTransform, apiConfigXTitleGetResponseSchemaExecute, apiConfigXTitleGetResponseSchemaExecuteTransform, apiConfigXTitleSetExecute, apiConfigXTitleSetExecuteTransform, apiConfigXTitleSetRequestSchemaExecute, apiConfigXTitleSetRequestSchemaExecuteTransform, apiConfigXTitleSetResponseSchemaExecute, apiConfigXTitleSetResponseSchemaExecuteTransform, apiKillExecute, apiKillExecuteTransform, apiKillRequestSchemaExecute, apiKillRequestSchemaExecuteTransform, apiKillResponseSchemaExecute, apiKillResponseSchemaExecuteTransform, apiSpawnExecute, apiSpawnExecuteTransform, apiSpawnRequestSchemaExecute, apiSpawnRequestSchemaExecuteTransform, apiSpawnResponseSchemaExecute, apiSpawnResponseSchemaExecuteTransform, dbConfigAddressGetExecute, dbConfigAddressGetExecuteTransform, dbConfigAddressGetRequestSchemaExecute, dbConfigAddressGetRequestSchemaExecuteTransform, dbConfigAddressGetResponseSchemaExecute, dbConfigAddressGetResponseSchemaExecuteTransform, dbConfigAddressSetExecute, dbConfigAddressSetExecuteTransform, dbConfigAddressSetRequestSchemaExecute, dbConfigAddressSetRequestSchemaExecuteTransform, dbConfigAddressSetResponseSchemaExecute, dbConfigAddressSetResponseSchemaExecuteTransform, dbConfigDatabaseGetExecute, dbConfigDatabaseGetExecuteTransform, dbConfigDatabaseGetRequestSchemaExecute, dbConfigDatabaseGetRequestSchemaExecuteTransform, dbConfigDatabaseGetResponseSchemaExecute, dbConfigDatabaseGetResponseSchemaExecuteTransform, dbConfigDatabaseSetExecute, dbConfigDatabaseSetExecuteTransform, dbConfigDatabaseSetRequestSchemaExecute, dbConfigDatabaseSetRequestSchemaExecuteTransform, dbConfigDatabaseSetResponseSchemaExecute, dbConfigDatabaseSetResponseSchemaExecuteTransform, dbConfigGetExecute, dbConfigGetExecuteTransform, dbConfigGetRequestSchemaExecute, dbConfigGetRequestSchemaExecuteTransform, dbConfigGetResponseSchemaExecute, dbConfigGetResponseSchemaExecuteTransform, dbConfigPasswordGetExecute, dbConfigPasswordGetExecuteTransform, dbConfigPasswordGetRequestSchemaExecute, dbConfigPasswordGetRequestSchemaExecuteTransform, dbConfigPasswordGetResponseSchemaExecute, dbConfigPasswordGetResponseSchemaExecuteTransform, dbConfigPasswordSetExecute, dbConfigPasswordSetExecuteTransform, dbConfigPasswordSetRequestSchemaExecute, dbConfigPasswordSetRequestSchemaExecuteTransform, dbConfigPasswordSetResponseSchemaExecute, dbConfigPasswordSetResponseSchemaExecuteTransform, dbConfigUserGetExecute, dbConfigUserGetExecuteTransform, dbConfigUserGetRequestSchemaExecute, dbConfigUserGetRequestSchemaExecuteTransform, dbConfigUserGetResponseSchemaExecute, dbConfigUserGetResponseSchemaExecuteTransform, dbConfigUserSetExecute, dbConfigUserSetExecuteTransform, dbConfigUserSetRequestSchemaExecute, dbConfigUserSetRequestSchemaExecuteTransform, dbConfigUserSetResponseSchemaExecute, dbConfigUserSetResponseSchemaExecuteTransform, dbKillExecute, dbKillExecuteTransform, dbKillRequestSchemaExecute, dbKillRequestSchemaExecuteTransform, dbKillResponseSchemaExecute, dbKillResponseSchemaExecuteTransform, dbQueryExecute, dbQueryExecuteTransform, dbQueryRequestSchemaExecute, dbQueryRequestSchemaExecuteTransform, dbQueryResponseSchemaExecute, dbQueryResponseSchemaExecuteTransform, dbSpawnExecute, dbSpawnExecuteTransform, dbSpawnRequestSchemaExecute, dbSpawnRequestSchemaExecuteTransform, dbSpawnResponseSchemaExecute, dbSpawnResponseSchemaExecuteTransform, functionsExecuteStandardExecute, functionsExecuteStandardExecuteStreaming, functionsExecuteStandardExecuteStreamingTransform, functionsExecuteStandardExecuteTransform, functionsExecuteStandardRequestSchemaExecute, functionsExecuteStandardRequestSchemaExecuteTransform, functionsExecuteStandardResponseSchemaExecute, functionsExecuteStandardResponseSchemaExecuteTransform, functionsExecuteSwissSystemExecute, functionsExecuteSwissSystemExecuteStreaming, functionsExecuteSwissSystemExecuteStreamingTransform, functionsExecuteSwissSystemExecuteTransform, functionsExecuteSwissSystemRequestSchemaExecute, functionsExecuteSwissSystemRequestSchemaExecuteTransform, functionsExecuteSwissSystemResponseSchemaExecute, functionsExecuteSwissSystemResponseSchemaExecuteTransform, functionsGetExecute, functionsGetExecuteTransform, functionsGetRequestSchemaExecute, functionsGetRequestSchemaExecuteTransform, functionsGetResponseSchemaExecute, functionsGetResponseSchemaExecuteTransform, functionsListExecute, functionsListExecuteTransform, functionsListRequestSchemaExecute, functionsListRequestSchemaExecuteTransform, functionsListResponseSchemaExecute, functionsListResponseSchemaExecuteTransform, functionsProfilesGetExecute, functionsProfilesGetExecuteTransform, functionsProfilesGetRequestSchemaExecute, functionsProfilesGetRequestSchemaExecuteTransform, functionsProfilesGetResponseSchemaExecute, functionsProfilesGetResponseSchemaExecuteTransform, functionsProfilesListExecute, functionsProfilesListExecuteTransform, functionsProfilesListRequestSchemaExecute, functionsProfilesListRequestSchemaExecuteTransform, functionsProfilesListResponseSchemaExecute, functionsProfilesListResponseSchemaExecuteTransform, functionsProfilesPublishExecute, functionsProfilesPublishExecuteTransform, functionsProfilesPublishRequestSchemaExecute, functionsProfilesPublishRequestSchemaExecuteTransform, functionsProfilesPublishResponseSchemaExecute, functionsProfilesPublishResponseSchemaExecuteTransform, functionsPublishExecute, functionsPublishExecuteTransform, functionsPublishRequestSchemaExecute, functionsPublishRequestSchemaExecuteTransform, functionsPublishResponseSchemaExecute, functionsPublishResponseSchemaExecuteTransform, invokeCliRequest, listen, mcpConfigAddressGetExecute, mcpConfigAddressGetExecuteTransform, mcpConfigAddressGetRequestSchemaExecute, mcpConfigAddressGetRequestSchemaExecuteTransform, mcpConfigAddressGetResponseSchemaExecute, mcpConfigAddressGetResponseSchemaExecuteTransform, mcpConfigAddressSetExecute, mcpConfigAddressSetExecuteTransform, mcpConfigAddressSetRequestSchemaExecute, mcpConfigAddressSetRequestSchemaExecuteTransform, mcpConfigAddressSetResponseSchemaExecute, mcpConfigAddressSetResponseSchemaExecuteTransform, mcpConfigGetExecute, mcpConfigGetExecuteTransform, mcpConfigGetRequestSchemaExecute, mcpConfigGetRequestSchemaExecuteTransform, mcpConfigGetResponseSchemaExecute, mcpConfigGetResponseSchemaExecuteTransform, mcpConfigPortGetExecute, mcpConfigPortGetExecuteTransform, mcpConfigPortGetRequestSchemaExecute, mcpConfigPortGetRequestSchemaExecuteTransform, mcpConfigPortGetResponseSchemaExecute, mcpConfigPortGetResponseSchemaExecuteTransform, mcpConfigPortSetExecute, mcpConfigPortSetExecuteTransform, mcpConfigPortSetRequestSchemaExecute, mcpConfigPortSetRequestSchemaExecuteTransform, mcpConfigPortSetResponseSchemaExecute, mcpConfigPortSetResponseSchemaExecuteTransform, mcpKillExecute, mcpKillExecuteTransform, mcpKillRequestSchemaExecute, mcpKillRequestSchemaExecuteTransform, mcpKillResponseSchemaExecute, mcpKillResponseSchemaExecuteTransform, mcpSpawnExecute, mcpSpawnExecuteTransform, mcpSpawnRequestSchemaExecute, mcpSpawnRequestSchemaExecuteTransform, mcpSpawnResponseSchemaExecute, mcpSpawnResponseSchemaExecuteTransform, pluginsGetExecute, pluginsGetExecuteTransform, pluginsGetRequestSchemaExecute, pluginsGetRequestSchemaExecuteTransform, pluginsGetResponseSchemaExecute, pluginsGetResponseSchemaExecuteTransform, pluginsInstallFilesystemExecute, pluginsInstallFilesystemExecuteTransform, pluginsInstallFilesystemRequestSchemaExecute, pluginsInstallFilesystemRequestSchemaExecuteTransform, pluginsInstallFilesystemResponseSchemaExecute, pluginsInstallFilesystemResponseSchemaExecuteTransform, pluginsInstallGithubExecute, pluginsInstallGithubExecuteTransform, pluginsInstallGithubRequestSchemaExecute, pluginsInstallGithubRequestSchemaExecuteTransform, pluginsInstallGithubResponseSchemaExecute, pluginsInstallGithubResponseSchemaExecuteTransform, pluginsListExecute, pluginsListExecuteTransform, pluginsListRequestSchemaExecute, pluginsListRequestSchemaExecuteTransform, pluginsListResponseSchemaExecute, pluginsListResponseSchemaExecuteTransform, pluginsRunExecute, pluginsRunExecuteTransform, pluginsRunRequestSchemaExecute, pluginsRunRequestSchemaExecuteTransform, pluginsRunResponseSchemaExecute, pluginsRunResponseSchemaExecuteTransform, swarmsGetExecute, swarmsGetExecuteTransform, swarmsGetRequestSchemaExecute, swarmsGetRequestSchemaExecuteTransform, swarmsGetResponseSchemaExecute, swarmsGetResponseSchemaExecuteTransform, swarmsListExecute, swarmsListExecuteTransform, swarmsListRequestSchemaExecute, swarmsListRequestSchemaExecuteTransform, swarmsListResponseSchemaExecute, swarmsListResponseSchemaExecuteTransform, swarmsPublishExecute, swarmsPublishExecuteTransform, swarmsPublishRequestSchemaExecute, swarmsPublishRequestSchemaExecuteTransform, swarmsPublishResponseSchemaExecute, swarmsPublishResponseSchemaExecuteTransform, tasksListExecute, tasksListExecuteTransform, tasksListRequestSchemaExecute, tasksListRequestSchemaExecuteTransform, tasksListResponseSchemaExecute, tasksListResponseSchemaExecuteTransform, tasksRunExecute, tasksRunExecuteTransform, tasksRunRequestSchemaExecute, tasksRunRequestSchemaExecuteTransform, tasksRunResponseSchemaExecute, tasksRunResponseSchemaExecuteTransform, tasksScheduleExecute, tasksScheduleExecuteTransform, tasksScheduleRequestSchemaExecute, tasksScheduleRequestSchemaExecuteTransform, tasksScheduleResponseSchemaExecute, tasksScheduleResponseSchemaExecuteTransform, toolsGetExecute, toolsGetExecuteTransform, toolsGetRequestSchemaExecute, toolsGetRequestSchemaExecuteTransform, toolsGetResponseSchemaExecute, toolsGetResponseSchemaExecuteTransform, toolsInstallFilesystemExecute, toolsInstallFilesystemExecuteTransform, toolsInstallFilesystemRequestSchemaExecute, toolsInstallFilesystemRequestSchemaExecuteTransform, toolsInstallFilesystemResponseSchemaExecute, toolsInstallFilesystemResponseSchemaExecuteTransform, toolsInstallGithubExecute, toolsInstallGithubExecuteTransform, toolsInstallGithubRequestSchemaExecute, toolsInstallGithubRequestSchemaExecuteTransform, toolsInstallGithubResponseSchemaExecute, toolsInstallGithubResponseSchemaExecuteTransform, toolsListExecute, toolsListExecuteTransform, toolsListRequestSchemaExecute, toolsListRequestSchemaExecuteTransform, toolsListResponseSchemaExecute, toolsListResponseSchemaExecuteTransform, toolsRunExecute, toolsRunExecuteTransform, toolsRunRequestSchemaExecute, toolsRunRequestSchemaExecuteTransform, toolsRunResponseSchemaExecute, toolsRunResponseSchemaExecuteTransform, updateExecute, updateExecuteTransform, updateRequestSchemaExecute, updateRequestSchemaExecuteTransform, updateResponseSchemaExecute, updateResponseSchemaExecuteTransform, viewerConfigAddressGetExecute, viewerConfigAddressGetExecuteTransform, viewerConfigAddressGetRequestSchemaExecute, viewerConfigAddressGetRequestSchemaExecuteTransform, viewerConfigAddressGetResponseSchemaExecute, viewerConfigAddressGetResponseSchemaExecuteTransform, viewerConfigAddressSetExecute, viewerConfigAddressSetExecuteTransform, viewerConfigAddressSetRequestSchemaExecute, viewerConfigAddressSetRequestSchemaExecuteTransform, viewerConfigAddressSetResponseSchemaExecute, viewerConfigAddressSetResponseSchemaExecuteTransform, viewerConfigGetExecute, viewerConfigGetExecuteTransform, viewerConfigGetRequestSchemaExecute, viewerConfigGetRequestSchemaExecuteTransform, viewerConfigGetResponseSchemaExecute, viewerConfigGetResponseSchemaExecuteTransform, viewerConfigSecretGetExecute, viewerConfigSecretGetExecuteTransform, viewerConfigSecretGetRequestSchemaExecute, viewerConfigSecretGetRequestSchemaExecuteTransform, viewerConfigSecretGetResponseSchemaExecute, viewerConfigSecretGetResponseSchemaExecuteTransform, viewerConfigSecretSetExecute, viewerConfigSecretSetExecuteTransform, viewerConfigSecretSetRequestSchemaExecute, viewerConfigSecretSetRequestSchemaExecuteTransform, viewerConfigSecretSetResponseSchemaExecute, viewerConfigSecretSetResponseSchemaExecuteTransform, viewerConfigSignatureGetExecute, viewerConfigSignatureGetExecuteTransform, viewerConfigSignatureGetRequestSchemaExecute, viewerConfigSignatureGetRequestSchemaExecuteTransform, viewerConfigSignatureGetResponseSchemaExecute, viewerConfigSignatureGetResponseSchemaExecuteTransform, viewerConfigSignatureSetExecute, viewerConfigSignatureSetExecuteTransform, viewerConfigSignatureSetRequestSchemaExecute, viewerConfigSignatureSetRequestSchemaExecuteTransform, viewerConfigSignatureSetResponseSchemaExecute, viewerConfigSignatureSetResponseSchemaExecuteTransform, viewerGenerateSecretSignaturePairExecute, viewerGenerateSecretSignaturePairExecuteTransform, viewerGenerateSecretSignaturePairRequestSchemaExecute, viewerGenerateSecretSignaturePairRequestSchemaExecuteTransform, viewerGenerateSecretSignaturePairResponseSchemaExecute, viewerGenerateSecretSignaturePairResponseSchemaExecuteTransform, viewerKillExecute, viewerKillExecuteTransform, viewerKillRequestSchemaExecute, viewerKillRequestSchemaExecuteTransform, viewerKillResponseSchemaExecute, viewerKillResponseSchemaExecuteTransform, viewerSendExecute, viewerSendExecuteTransform, viewerSendRequestSchemaExecute, viewerSendRequestSchemaExecuteTransform, viewerSendResponseSchemaExecute, viewerSendResponseSchemaExecuteTransform, viewerSpawnExecute, viewerSpawnExecuteTransform, viewerSpawnRequestSchemaExecute, viewerSpawnRequestSchemaExecuteTransform, viewerSpawnResponseSchemaExecute, viewerSpawnResponseSchemaExecuteTransform };
