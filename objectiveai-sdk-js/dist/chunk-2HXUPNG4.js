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
}).describe("Host \u2192 iframe. Carries data into the plugin (the existing\npath). `sub_type` is the snake_case discriminator the plugin\nlistens on (built-ins: `agent_completions` /\n`functions_executions` / `functions_inventions_recursive` /\n`laboratories_executions`; plugins: whatever they declared in\ntheir manifest's `viewer_routes[i].type`).").meta({ "variantTitle": "Inbound" }), z.object({
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
var RemotePathSchema = z.union([z.object({
  commit: z.string(),
  owner: z.string(),
  remote: z.literal("github"),
  repository: z.string()
}).meta({ "variantTitle": "Github" }), z.object({
  commit: z.string(),
  owner: z.string(),
  remote: z.literal("filesystem"),
  repository: z.string()
}).meta({ "variantTitle": "Filesystem" }), z.object({
  name: z.string(),
  remote: z.literal("mock")
}).meta({ "variantTitle": "Mock" })]).meta({ title: "RemotePath" });

// src/agent/getAgentResponse.ts
var AgentGetAgentResponseSchema = RemotePathSchema.and(z.object({})).describe("Response containing a single Agent with creation timestamp.").meta({ title: "agent.GetAgentResponse" });
var CliErrorTypeSchema = z.literal("error").describe('Single-variant discriminator for [`Error`]\'s `type` field.\nAlways `"error"` on the wire.').meta({ title: "cli.ErrorType" });
var CliLevelSchema = z.enum(["trace", "debug", "info", "warn", "error"]).describe("Severity matching the conventions used by bunyan / pino / `log` crate\nJSON encoders. `fatal` is encoded separately on [`Error`].").meta({ title: "cli.Level" });

// src/cli/error.ts
var CliErrorSchema = z.object({
  fatal: z.boolean().nullable().meta({ omitempty: true }).optional(),
  level: CliLevelSchema.nullable().meta({ omitempty: true }).optional(),
  message: JsonValueSchema,
  type: CliErrorTypeSchema
}).describe('A failure or advisory written to stdout. `fatal: true` means the\nprocess is exiting with a non-zero status; `fatal: false` is a\nnon-blocking warning (e.g. auto-update failed but the requested\ncommand still ran).\n\n`message` is an arbitrary JSON value so producers can emit\nstructured payloads (e.g. `{"code": ..., "detail": ...}`). Wrap\na plain string as `Value::String(...)` (or use `.into()`) and the\nwire bytes stay identical to the old `String`-only shape.\n\nThe `type` field is a single-variant `ErrorType` enum that\nalways serializes to `"error"`. This is what disambiguates the\nuntagged `Output` enum from a notification \u2014 `Output` tries\n`Error` first, and the constant `type:"error"` tag is what\nrejects every non-error wire shape.').meta({ title: "cli.Error" });

// src/viewer/command/agents/get.ts
async function agentsGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/get" }), z.union([CliErrorSchema, AgentGetAgentResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents get: cli produced no output before the end marker");
  }
  return first;
}
async function agentsGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents get: cli produced no output before the end marker");
  }
  return first;
}
async function agentsGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandAgentsListActiveResponseItemSchema = z.object({
  agent_id: z.string(),
  last_log: z.number().int().min(0).max(18446744073709552e3)
}).meta({ title: "cli.command.agents.list.active.ResponseItem" });

// src/viewer/command/agents/list/active.ts
function agentsListActiveExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/list/active" }), z.union([CliErrorSchema, CliCommandAgentsListActiveResponseItemSchema]));
}
function agentsListActiveExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/list/active" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function agentsListActiveRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/list/active/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents list active request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsListActiveRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/list/active/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents list active request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsListActiveResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/list/active/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents list active response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsListActiveResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/list/active/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents list active response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandAgentsListAvailableResponseFavoriteSchema = z.union([z.object({
  commit: z.string().nullable().optional(),
  name: z.string(),
  note: z.string(),
  owner: z.string(),
  remote: z.literal("github"),
  repository: z.string()
}).meta({ "variantTitle": "Github" }), z.object({
  commit: z.string().nullable().optional(),
  name: z.string(),
  note: z.string(),
  owner: z.string(),
  remote: z.literal("filesystem"),
  repository: z.string()
}).meta({ "variantTitle": "Filesystem" }), z.object({
  name: z.string(),
  note: z.string(),
  remote: z.literal("mock")
}).meta({ "variantTitle": "Mock" })]).meta({ title: "cli.command.agents.list.available.ResponseFavorite" });

// src/cli/command/agents/list/available/responseItem.ts
var CliCommandAgentsListAvailableResponseItemSchema = z.union([CliCommandAgentsListAvailableResponseFavoriteSchema.meta({ "title": "cli.command.agents.list.available.ResponseFavorite", "variantTitle": "Favorite" }), RemotePathSchema.meta({ "title": "RemotePath", "variantTitle": "Item" })]).meta({ title: "cli.command.agents.list.available.ResponseItem" });

// src/viewer/command/agents/list/available.ts
function agentsListAvailableExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/list/available" }), z.union([CliErrorSchema, CliCommandAgentsListAvailableResponseItemSchema]));
}
function agentsListAvailableExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/list/available" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function agentsListAvailableRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/list/available/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents list available request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsListAvailableRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/list/available/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents list available request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsListAvailableResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/list/available/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents list available response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsListAvailableResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/list/available/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents list available response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandAgentsMeResponseSchema = z.object({
  agent_full_id: z.string().nullable().meta({ omitempty: true }).optional(),
  agent_id: z.string().nullable().meta({ omitempty: true }).optional(),
  agent_instance_hierarchy: z.string(),
  agent_remote: z.string().nullable().meta({ omitempty: true }).optional(),
  agent_tags: z.array(z.string()).describe("Every tag currently BOUND to `agent_instance_hierarchy`,\nnewest-bound first. Empty `[]` when the cli's hierarchy\ncarries no tags. Always emitted so callers can rely on the\nfield being present.")
}).meta({ title: "cli.command.agents.me.Response" });

// src/viewer/command/agents/me.ts
async function agentsMeExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/me" }), z.union([CliErrorSchema, CliCommandAgentsMeResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents me: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMeExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/me" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents me: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMeRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/me/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents me request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMeRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/me/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents me request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMeResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/me/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents me response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMeResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/me/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents me response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandAgentsMessageResponseSchema = z.union([z.object({
  agent_instance_hierarchy: z.string(),
  response_id: z.string()
}).meta({ "variantTitle": "Queued" }), z.object({
  agent_instance_hierarchy: z.string()
}).meta({ "variantTitle": "Delivered" })]).meta({ title: "cli.command.agents.message.Response" });
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

// src/agent/completions/message/richContentPart.ts
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
  role: AgentCompletionsResponseAssistantRoleSchema,
  service_tier: z.string().nullable().meta({ omitempty: true }).optional(),
  system_fingerprint: z.string().nullable().meta({ omitempty: true }).optional(),
  tool_calls: z.array(AgentCompletionsMessageAssistantToolCallDeltaSchema).nullable().meta({ omitempty: true }).optional(),
  upstream_id: z.string(),
  usage: AgentCompletionsResponseUpstreamUsageSchema.nullable().describe("Upstream usage for this assistant response (set by upstream clients).").meta({ omitempty: true }).optional()
}).describe("A chunk of a streaming agent completion response.\n\nMultiple chunks are received via Server-Sent Events and can be\naccumulated into a complete [`AgentCompletion`](response::unary::AgentCompletion)\nusing the [`push`](Self::push) method.").meta({ title: "agent.completions.response.streaming.AssistantResponseChunk" });
var AgentCompletionsMessageToolResponseMetadataSchema = z.object({
  notifications: z.number().int().min(0).max(18446744073709552e3).nullable().describe("Count of pending notifications the proxy drained and prepended\nto the tool response's `content` before returning. Only set\nwhen at least one notification was drained.").meta({ omitempty: true }).optional()
}).describe("Vendor-extension metadata attached to a tool response. The\n`objectiveai-mcp-proxy` populates known keys (currently\n`notifications`); the SDK lossy-decodes the MCP `_meta` bag into\nthis typed shape. Unknown keys are dropped.").meta({ title: "agent.completions.message.ToolResponseMetadata" });
var AgentCompletionsResponseToolRoleSchema = z.literal("tool").meta({ title: "agent.completions.response.ToolRole" });

// src/agent/completions/response/toolResponse.ts
var AgentCompletionsResponseToolResponseSchema = z.object({
  content: AgentCompletionsMessageRichContentSchema.describe("The content of the tool response."),
  index: z.number().int().min(0).max(18446744073709552e3),
  metadata: AgentCompletionsMessageToolResponseMetadataSchema.nullable().describe("Optional vendor-extension metadata, populated by\n`objectiveai-mcp-proxy` via MCP's `_meta` extension bag.").meta({ omitempty: true }).optional(),
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

// src/cli/command/agents/message/responseItem.ts
var CliCommandAgentsMessageResponseItemSchema = z.union([AgentCompletionsResponseStreamingAgentCompletionChunkSchema.meta({ "title": "agent.completions.response.streaming.AgentCompletionChunk", "variantTitle": "Chunk" }), z.object({
  agent_instance_hierarchy: z.string(),
  response_id: z.string()
}).meta({ "variantTitle": "Queued" }), z.object({
  agent_instance_hierarchy: z.string()
}).meta({ "variantTitle": "Delivered" })]).describe("Streamed-mode wire shape for `agents message`. Emitted as one\nJSON-line per item on the cli's stdout when\n`Request::dangerous_advanced.stream = Some(true)`. Untagged shape\nis forward-compatible with [`Response`]: in stream mode, item 0 is\nalways a `Queued` or `Delivered` variant carrying the same fields\nas the unary `Response::Queued` / `Response::Delivered`. Under the\n`Queued` path with streaming on, item 0 is followed by zero or\nmore `Chunk` items until the spawned instance-runner's stdout\nEOFs.").meta({ title: "cli.command.agents.message.ResponseItem" });

// src/viewer/command/agents/message.ts
function agentsMessageExecuteStreaming(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, dangerous_advanced: { ...request.dangerous_advanced ?? {}, stream: true }, path_type: "agents/message" }), z.union([CliErrorSchema, CliCommandAgentsMessageResponseItemSchema]));
}
function agentsMessageExecuteStreamingJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, dangerous_advanced: { ...request.dangerous_advanced ?? {}, stream: true }, path_type: "agents/message" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function agentsMessageExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, dangerous_advanced: request.dangerous_advanced ? { ...request.dangerous_advanced, stream: void 0 } : request.dangerous_advanced, path_type: "agents/message" }), z.union([CliErrorSchema, CliCommandAgentsMessageResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents message: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMessageExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, dangerous_advanced: request.dangerous_advanced ? { ...request.dangerous_advanced, stream: void 0 } : request.dangerous_advanced, path_type: "agents/message" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents message: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMessageRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/message/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents message request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMessageRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/message/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents message request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMessageResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/message/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents message response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMessageResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/message/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents message response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandAgentsMessageQueueAddResponseSchema = z.object({
  agent_instance_hierarchy: z.string().nullable().meta({ omitempty: true }).optional(),
  agent_tag: z.string().nullable().meta({ omitempty: true }).optional(),
  id: z.number().int().min(-9223372036854776e3).max(9223372036854776e3)
}).describe("`id` is the row id from `tags.sqlite`'s `prompts` table. Exactly\none of `agent_instance_hierarchy` / `agent_tag` is set, matching\nthe chosen [`Target`] variant \xC3\xA2\xE2\u201A\xAC\xE2\u20AC\x9D `agent_instance_hierarchy` is\nthe **resolved** `{parent}/{instance}` for Direct mode.").meta({ title: "cli.command.agents.message_queue.add.Response" });

// src/viewer/command/agents/message_queue/add.ts
async function agentsMessageQueueAddExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/message-queue/add" }), z.union([CliErrorSchema, CliCommandAgentsMessageQueueAddResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents message_queue add: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMessageQueueAddExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/message-queue/add" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents message_queue add: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMessageQueueAddRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/message-queue/add/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents message_queue add request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMessageQueueAddRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/message-queue/add/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents message_queue add request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMessageQueueAddResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/message-queue/add/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents message_queue add response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMessageQueueAddResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/message-queue/add/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents message_queue add response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandAgentsMessageQueueDeleteResponseSchema = z.object({
  agent_instance_hierarchy: z.string().nullable().meta({ omitempty: true }).optional(),
  agent_tag: z.string().nullable().meta({ omitempty: true }).optional(),
  content: AgentCompletionsMessageRichContentSchema,
  enqueued_at: z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  id: z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  key: z.string().nullable().describe("Idempotency token, if the dropped row had one.").meta({ omitempty: true }).optional()
}).describe("What was deleted. Carries every column of the original\n`prompts` row so the caller can confirm the drop:\nexactly one of `agent_instance_hierarchy` / `agent_tag` is set\n(matching the original target), `enqueued_at` is the original\nunix-seconds timestamp, and `content` is the reconstructed\n`RichContent` body.").meta({ title: "cli.command.agents.message_queue.delete.Response" });

// src/viewer/command/agents/message_queue/delete.ts
async function agentsMessageQueueDeleteExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/message-queue/delete" }), z.union([CliErrorSchema, CliCommandAgentsMessageQueueDeleteResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents message_queue delete: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMessageQueueDeleteExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/message-queue/delete" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents message_queue delete: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMessageQueueDeleteRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/message-queue/delete/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents message_queue delete request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMessageQueueDeleteRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/message-queue/delete/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents message_queue delete request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMessageQueueDeleteResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/message-queue/delete/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents message_queue delete response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMessageQueueDeleteResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/message-queue/delete/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents message_queue delete response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandAgentsMessageQueueDeliverResponseItemSchema = z.object({
  agent_instance_hierarchy: z.string(),
  agent_tag: z.string().nullable().meta({ omitempty: true }).optional(),
  item: CliCommandAgentsMessageResponseItemSchema
}).describe("One item from one of the fanned-out `agents message` calls,\naugmented with the resolved target that produced it.\n\n`item` is the inner `agents::message::ResponseItem` \u2014\n`Queued` / `Delivered` / `Chunk`. `agent_instance_hierarchy`\nis the resolved hierarchy the delivery was addressed to;\n`agent_tag` is `Some` only when the underlying queue row was\nTag-addressed (with the tag now resolved to that hierarchy\nvia the BOUND lookup).").meta({ title: "cli.command.agents.message_queue.deliver.ResponseItem" });

// src/viewer/command/agents/message_queue/deliver.ts
function agentsMessageQueueDeliverExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/message-queue/deliver" }), z.union([CliErrorSchema, CliCommandAgentsMessageQueueDeliverResponseItemSchema]));
}
function agentsMessageQueueDeliverExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/message-queue/deliver" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function agentsMessageQueueDeliverRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/message-queue/deliver/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents message_queue deliver request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMessageQueueDeliverRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/message-queue/deliver/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents message_queue deliver request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMessageQueueDeliverResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/message-queue/deliver/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents message_queue deliver response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMessageQueueDeliverResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/message-queue/deliver/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents message_queue deliver response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMessageQueueReadIdExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/message-queue/read/id" }), z.union([CliErrorSchema, AgentCompletionsMessageRichContentPartSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents message_queue read id: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMessageQueueReadIdExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/message-queue/read/id" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents message_queue read id: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMessageQueueReadIdRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/message-queue/read/id/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents message_queue read id request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMessageQueueReadIdRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/message-queue/read/id/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents message_queue read id request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMessageQueueReadIdResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/message-queue/read/id/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents message_queue read id response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMessageQueueReadIdResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/message-queue/read/id/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents message_queue read id response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandAgentsReadAllResponseContentSchema = z.union([z.number().int().min(-9223372036854776e3).max(9223372036854776e3).meta({ "variantTitle": "One" }), z.array(z.number().int().min(-9223372036854776e3).max(9223372036854776e3)).meta({ "variantTitle": "Many" })]).meta({ title: "cli.command.agents.read.all.ResponseContent" });

// src/cli/command/agents/message_queue/read/pending/responseItem.ts
var CliCommandAgentsMessageQueueReadPendingResponseItemSchema = z.union([z.object({
  agent_instance: z.string(),
  by: z.literal("agent_instance"),
  content: CliCommandAgentsReadAllResponseContentSchema,
  id: z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  key: z.string().nullable().describe("Idempotency token, if the row was enqueued with `--key`.\nSkipped from the wire shape when `None`.").meta({ omitempty: true }).optional()
}).meta({ "variantTitle": "AgentInstance" }), z.union([z.object({
  agent_instance_hierarchy: z.string(),
  agent_tag: z.string(),
  by: z.literal("tag"),
  content: CliCommandAgentsReadAllResponseContentSchema,
  id: z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  key: z.string().nullable().describe("Idempotency token, if the row was enqueued with `--key`.").meta({ omitempty: true }).optional(),
  state: z.literal("bound")
}).meta({ "variantTitle": "Bound" }), z.object({
  agent_full_id: z.string(),
  agent_tag: z.string(),
  by: z.literal("tag"),
  content: CliCommandAgentsReadAllResponseContentSchema,
  id: z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  key: z.string().nullable().describe("Idempotency token, if the row was enqueued with `--key`.").meta({ omitempty: true }).optional(),
  parent_agent_instance_hierarchy: z.string(),
  state: z.literal("pending")
}).meta({ "variantTitle": "Pending" })]).describe('2-state result of a successful tag-name lookup. PENDING surfaces\nthe pre-spawn `(parent, agent_full_id)` pair so callers can see\nwhy the tag exists without a hierarchy yet. The third "not\nregistered" possibility is represented at the [`Response`]\nlevel via [`Response::Absent`], not as a variant here.').meta({ "variantTitle": "Tag" })]).describe('One queued prompt. Direct rows carry only the bare\n`agent_instance` (= leaf segment of the hierarchy); Tag rows\ncarry the literal tag name and flatten the joined 2-state\nstatus onto the same JSON object \xC3\u0192\xC6\u2019\xC3\u201A\xC2\xA2\xC3\u0192\xC2\xA2\xC3\xA2\xE2\u201A\xAC\xC5\xA1\xC3\u201A\xC2\xAC\xC3\u0192\xC2\xA2\xC3\xA2\xE2\u20AC\u0161\xC2\xAC\xC3\u201A\xC2\x9D yielding e.g.\n`{"by":"tag","id":42,"agent_tag":"foo","state":"bound","agent_instance_hierarchy":"\xC3\u0192\xC6\u2019\xC3\u201A\xC2\xA2\xC3\u0192\xC2\xA2\xC3\xA2\xE2\u201A\xAC\xC5\xA1\xC3\u201A\xC2\xAC\xC3\u0192\xE2\u20AC\u0161\xC3\u201A\xC2\xA6",\n"content":17}` (single-part) or `\xC3\u0192\xC6\u2019\xC3\u201A\xC2\xA2\xC3\u0192\xC2\xA2\xC3\xA2\xE2\u201A\xAC\xC5\xA1\xC3\u201A\xC2\xAC\xC3\u0192\xE2\u20AC\u0161\xC3\u201A\xC2\xA6"content":[17,18,19]"` (multi-\npart) rather than nesting the state under its own object.\n\nBoth variants carry the resolved content body as a\n[`super::super::super::read::all::ResponseContent`] \xC3\u0192\xC6\u2019\xC3\u201A\xC2\xA2\xC3\u0192\xC2\xA2\xC3\xA2\xE2\u201A\xAC\xC5\xA1\xC3\u201A\xC2\xAC\xC3\u0192\xC2\xA2\xC3\xA2\xE2\u20AC\u0161\xC2\xAC\xC3\u201A\xC2\x9D `One(i64)` for a\nsingle-part `RichContent::Text` (or single-element\n`RichContent::Parts`), `Many(Vec<i64>)` for multi-part payloads.\nEach id is a `prompt_contents.id` resolvable via\n`agents message-queue read id`.').meta({ title: "cli.command.agents.message_queue.read.pending.ResponseItem" });

// src/viewer/command/agents/message_queue/read/pending.ts
function agentsMessageQueueReadPendingExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/message-queue/read/pending" }), z.union([CliErrorSchema, CliCommandAgentsMessageQueueReadPendingResponseItemSchema]));
}
function agentsMessageQueueReadPendingExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/message-queue/read/pending" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function agentsMessageQueueReadPendingRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/message-queue/read/pending/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents message_queue read pending request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMessageQueueReadPendingRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/message-queue/read/pending/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents message_queue read pending request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMessageQueueReadPendingResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/message-queue/read/pending/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents message_queue read pending response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsMessageQueueReadPendingResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/message-queue/read/pending/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents message_queue read pending response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandAgentsPublishResponseSchema = z.object({
  sha: z.string()
}).meta({ title: "cli.command.agents.publish.Response" });

// src/viewer/command/agents/publish.ts
async function agentsPublishExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/publish" }), z.union([CliErrorSchema, CliCommandAgentsPublishResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents publish: cli produced no output before the end marker");
  }
  return first;
}
async function agentsPublishExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/publish" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents publish: cli produced no output before the end marker");
  }
  return first;
}
async function agentsPublishRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/publish/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents publish request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsPublishRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/publish/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents publish request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsPublishResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/publish/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents publish response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsPublishResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/publish/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents publish response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandAgentsReadAllResponseQueueMessageSchema = z.union([z.object({
  content: CliCommandAgentsReadAllResponseContentSchema,
  name: z.string().nullable().meta({ omitempty: true }).optional(),
  type: z.literal("developer")
}).meta({ "variantTitle": "Developer" }), z.object({
  content: CliCommandAgentsReadAllResponseContentSchema,
  name: z.string().nullable().meta({ omitempty: true }).optional(),
  type: z.literal("system")
}).meta({ "variantTitle": "System" }), z.object({
  content: CliCommandAgentsReadAllResponseContentSchema,
  name: z.string().nullable().meta({ omitempty: true }).optional(),
  type: z.literal("user")
}).meta({ "variantTitle": "User" }), z.object({
  content: CliCommandAgentsReadAllResponseContentSchema.nullable().meta({ omitempty: true }).optional(),
  name: z.string().nullable().meta({ omitempty: true }).optional(),
  reasoning: z.number().int().min(-9223372036854776e3).max(9223372036854776e3).nullable().meta({ omitempty: true }).optional(),
  refusal: z.number().int().min(-9223372036854776e3).max(9223372036854776e3).nullable().meta({ omitempty: true }).optional(),
  tool_calls: z.array(z.number().int().min(-9223372036854776e3).max(9223372036854776e3)).nullable().meta({ omitempty: true }).optional(),
  type: z.literal("assistant")
}).meta({ "variantTitle": "Assistant" }), z.object({
  content: CliCommandAgentsReadAllResponseContentSchema,
  tool_call_id: z.string(),
  type: z.literal("tool")
}).meta({ "variantTitle": "Tool" })]).meta({ title: "cli.command.agents.read.all.ResponseQueueMessage" });

// src/cli/command/agents/read/all/responseQueueItem.ts
var CliCommandAgentsReadAllResponseQueueItemSchema = z.union([z.object({
  content: CliCommandAgentsReadAllResponseContentSchema.nullable().meta({ omitempty: true }).optional(),
  reasoning: z.number().int().min(-9223372036854776e3).max(9223372036854776e3).nullable().meta({ omitempty: true }).optional(),
  refusal: z.number().int().min(-9223372036854776e3).max(9223372036854776e3).nullable().meta({ omitempty: true }).optional(),
  tool_calls: z.array(z.number().int().min(-9223372036854776e3).max(9223372036854776e3)).nullable().meta({ omitempty: true }).optional(),
  type: z.literal("assistant_response")
}).meta({ "variantTitle": "AssistantResponse" }), z.object({
  content: CliCommandAgentsReadAllResponseContentSchema,
  tool_call_id: z.string(),
  type: z.literal("tool_response")
}).meta({ "variantTitle": "ToolResponse" }), z.object({
  content: CliCommandAgentsReadAllResponseContentSchema,
  type: z.literal("notification")
}).meta({ "variantTitle": "Notification" }), z.object({
  messages: z.array(CliCommandAgentsReadAllResponseQueueMessageSchema),
  type: z.literal("agent_completion_request")
}).meta({ "variantTitle": "AgentCompletionRequest" }), z.object({
  id: z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  type: z.literal("function_execution_request")
}).meta({ "variantTitle": "FunctionExecutionRequest" }), z.object({
  id: z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  type: z.literal("function_invention_recursive_request")
}).meta({ "variantTitle": "FunctionInventionRecursiveRequest" })]).meta({ title: "cli.command.agents.read.all.ResponseQueueItem" });

// src/cli/command/agents/read/all/responseItem.ts
var CliCommandAgentsReadAllResponseItemSchema = z.object({
  agent_id: z.string(),
  items: z.array(CliCommandAgentsReadAllResponseQueueItemSchema)
}).meta({ title: "cli.command.agents.read.all.ResponseItem" });

// src/viewer/command/agents/read/all.ts
function agentsReadAllExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/read/all" }), z.union([CliErrorSchema, CliCommandAgentsReadAllResponseItemSchema]));
}
function agentsReadAllExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/read/all" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function agentsReadAllRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/read/all/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents read all request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsReadAllRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/read/all/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents read all request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsReadAllResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/read/all/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents read all response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsReadAllResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/read/all/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents read all response_schema: cli produced no output before the end marker");
  }
  return first;
}
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
var LogReferenceTagSchema = z.literal("reference").describe('Constant `"reference"` discriminator \u2014 the `"type"` field on every\n`LogReference` variant.').meta({ title: "LogReferenceTag" });

// src/logReference.ts
var LogReferenceSchema = z.object({
  path: z.string().describe("Relative on-disk path of the referenced file (under\n`${config_base_dir}/logs/`). Skipped when empty \u2014 the no-data\nsentinel case used by some wrappers when the inner chunk has\nno content to log.").meta({ omitempty: true }),
  type: LogReferenceTagSchema
}).describe("Plain on-disk pointer (`type` + `path` only).").meta({ title: "LogReference" });

// src/agent/completions/message/richContentLog.ts
var AgentCompletionsMessageRichContentLogSchema = z.union([LogReferenceSchema.meta({ "title": "LogReference", "variantTitle": "Reference" }), z.array(LogReferenceSchema).meta({ "variantTitle": "Parts" })]).meta({ title: "agent.completions.message.RichContentLog" });

// src/agent/completions/message/assistantMessageLog.ts
var AgentCompletionsMessageAssistantMessageLogSchema = z.object({
  content: AgentCompletionsMessageRichContentLogSchema.nullable().meta({ omitempty: true }).optional(),
  name: z.string().nullable().meta({ omitempty: true }).optional(),
  reasoning: LogReferenceSchema.nullable().meta({ omitempty: true }).optional(),
  refusal: LogReferenceSchema.nullable().meta({ omitempty: true }).optional(),
  tool_calls: z.array(LogReferenceSchema).nullable().meta({ omitempty: true }).optional()
}).meta({ title: "agent.completions.message.AssistantMessageLog" });
var AgentCompletionsMessageSimpleContentLogSchema = z.union([LogReferenceSchema.meta({ "title": "LogReference", "variantTitle": "Reference" }), z.array(LogReferenceSchema).meta({ "variantTitle": "Parts" })]).meta({ title: "agent.completions.message.SimpleContentLog" });

// src/agent/completions/message/developerMessageLog.ts
var AgentCompletionsMessageDeveloperMessageLogSchema = z.object({
  content: AgentCompletionsMessageSimpleContentLogSchema,
  name: z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "agent.completions.message.DeveloperMessageLog" });
var AgentCompletionsMessageSystemMessageLogSchema = z.object({
  content: AgentCompletionsMessageSimpleContentLogSchema,
  name: z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "agent.completions.message.SystemMessageLog" });
var AgentCompletionsMessageToolMessageLogSchema = z.object({
  content: AgentCompletionsMessageRichContentLogSchema,
  metadata: AgentCompletionsMessageToolResponseMetadataSchema.nullable().meta({ omitempty: true }).optional(),
  tool_call_id: z.string()
}).meta({ title: "agent.completions.message.ToolMessageLog" });
var AgentCompletionsMessageUserMessageLogSchema = z.object({
  content: AgentCompletionsMessageRichContentLogSchema,
  name: z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "agent.completions.message.UserMessageLog" });

// src/agent/completions/message/messageLog.ts
var AgentCompletionsMessageMessageLogSchema = z.union([AgentCompletionsMessageDeveloperMessageLogSchema.and(z.object({
  role: z.literal("developer")
})).meta({ "variantTitle": "Developer" }), AgentCompletionsMessageSystemMessageLogSchema.and(z.object({
  role: z.literal("system")
})).meta({ "variantTitle": "System" }), AgentCompletionsMessageUserMessageLogSchema.and(z.object({
  role: z.literal("user")
})).meta({ "variantTitle": "User" }), AgentCompletionsMessageAssistantMessageLogSchema.and(z.object({
  role: z.literal("assistant")
})).meta({ "variantTitle": "Assistant" }), AgentCompletionsMessageToolMessageLogSchema.and(z.object({
  role: z.literal("tool")
})).meta({ "variantTitle": "Tool" })]).meta({ title: "agent.completions.message.MessageLog" });
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
var AgentMockModeSchema = z.enum(["default", "invention", "laboratory_builder", "laboratory_evaluation"]).meta({ title: "agent.mock.Mode" });
var AgentMockOutputModeSchema = z.union([z.literal("instruction").describe("The model is instructed via the prompt to output a specific key.\n\nThis is the default and most widely supported mode.").meta({ "variantTitle": "Instruction" }), z.literal("json_schema").describe("A JSON schema response format is used with an enum of possible keys.\n\nRequires model support for structured JSON output.").meta({ "variantTitle": "JsonSchema" }), z.literal("tool_call").describe("A forced tool call with an argument schema containing possible keys.\n\nRequires model support for tool/function calling.").meta({ "variantTitle": "ToolCall" })]).describe("The method used to constrain LLM output to valid response keys.\n\nIn vector completions, the model must select from a predefined set of\nresponses. This enum controls *how* that constraint is enforced.\n\n**Note:** This setting is only relevant for vector completions and is\ncompletely ignored for agent completions.").meta({ title: "agent.mock.OutputMode" });
var AgentMockUpstreamSchema = z.literal("mock").describe("Mock upstream marker.").meta({ title: "agent.mock.Upstream" });

// src/agent/mock/agentBase.ts
var AgentMockAgentBaseSchema = z.object({
  calls: z.array(AgentMockCallSchema).nullable().describe("Deterministic-script override. When `Some`, the mock agent\nemits each [`super::Call`] as its own assistant turn \u2014\n`tool_calls` first, then `content` \u2014 in array order. Each\nsubsequent turn inspects the continuation to count how many\n`Call`s have already been satisfied (assistant message with\nexactly that `Call`'s `tool_calls` (by name+arguments) and\n`content`); the next un-matched `Call` is what that turn\nemits. Once every `Call` has been satisfied in the\ncontinuation, the mock falls through to its normal mode-driven\ndispatcher. Pure addition \u2014 agents without `calls` are\nunaffected.").meta({ omitempty: true }).optional(),
  client_objectiveai_mcp: AgentClientObjectiveaiMcpSchema.nullable().describe("Client-side ObjectiveAI MCP surface the calling client is\nexpected to expose locally back to the API (objectiveai\nbuilt-in, plus specific plugins / tools by owner+name+version).").meta({ omitempty: true }).optional(),
  error: z.boolean().nullable().describe("If true, the mock client will return an error instead of a response.").meta({ omitempty: true }).optional(),
  error_probability: z.number().int().min(0).max(255).nullable().describe("Probability (0-100) that the mock returns an error mid-stream.\nRequires `error` to be `Some(true)`.").meta({ omitempty: true }).optional(),
  mcp_servers: z.array(AgentMcpServerSchema).nullable().describe("MCP servers the agent can connect to.").meta({ omitempty: true }).optional(),
  mode: AgentMockModeSchema.nullable().describe("Mock agent mode. Defaults to `default`.").meta({ omitempty: true }).optional(),
  output_mode: AgentMockOutputModeSchema.describe("The output mode for vector completions. Ignored for agent completions."),
  top_logprobs: z.number().int().min(0).max(18446744073709552e3).nullable().describe("Number of top log probabilities to return (2-20).\n\n**Vector completions only.** Ignored for agent completions.").meta({ omitempty: true }).optional(),
  upstream: AgentMockUpstreamSchema.describe("The upstream provider marker.")
}).describe("The base configuration for a Mock Agent (without computed ID).").meta({ title: "agent.mock.AgentBase" });
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
  remote: z.literal("filesystem"),
  repository: z.string()
}).meta({ "variantTitle": "Filesystem" }), z.object({
  name: z.string(),
  remote: z.literal("mock")
}).meta({ "variantTitle": "Mock" })]).meta({ title: "RemotePathCommitOptional" });

// src/agent/inlineAgentBaseWithFallbacksOrRemoteCommitOptional.ts
var AgentInlineAgentBaseWithFallbacksOrRemoteCommitOptionalSchema = z.union([AgentInlineAgentBaseWithFallbacksSchema.meta({ "title": "agent.InlineAgentBaseWithFallbacks", "variantTitle": "AgentBase" }), RemotePathCommitOptionalSchema.meta({ "title": "RemotePathCommitOptional", "variantTitle": "Remote" })]).describe("Like [`InlineAgentBaseWithFallbacksOrRemote`] but with optional commit.\nUsed in request types where commit resolution happens server-side.").meta({ title: "agent.InlineAgentBaseWithFallbacksOrRemoteCommitOptional" });

// src/agent/completions/request/agentCompletionCreateParamsLog.ts
var AgentCompletionsRequestAgentCompletionCreateParamsLogSchema = z.object({
  agent: AgentInlineAgentBaseWithFallbacksOrRemoteCommitOptionalSchema,
  continuation: LogReferenceSchema.nullable().meta({ omitempty: true }).optional(),
  messages: z.array(LogReferenceSchema),
  provider: AgentCompletionsRequestProviderSchema.nullable().meta({ omitempty: true }).optional(),
  response_format: LogReferenceSchema.nullable().meta({ omitempty: true }).optional(),
  seed: z.number().int().min(-9223372036854776e3).max(9223372036854776e3).nullable().meta({ omitempty: true }).optional(),
  stream: z.boolean().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "agent.completions.request.AgentCompletionCreateParamsLog" });
var AgentCompletionsResponseStreamingRoleSchema = z.enum(["assistant", "tool"]).describe("The role of a referenced response-side message envelope \u2014 matches\nthe `role` tag serialized inside the referenced file\n(`AssistantResponseChunkLog` / `ToolResponseLog`).").meta({ title: "agent.completions.response.streaming.Role" });

// src/agent/completions/response/streaming/logReference.ts
var AgentCompletionsResponseStreamingLogReferenceSchema = z.object({
  index: z.number().int().min(0).max(18446744073709552e3).describe("The message's index within the completion \u2014 the message-index\nargument to the role-specific read commands."),
  path: z.string().meta({ omitempty: true }),
  role: AgentCompletionsResponseStreamingRoleSchema.describe("Which role subdir the file lives in, i.e. which command family\nreads it."),
  type: LogReferenceTagSchema
}).meta({ title: "agent.completions.response.streaming.LogReference" });

// src/agent/completions/response/streaming/agentCompletionChunkLog.ts
var AgentCompletionsResponseStreamingAgentCompletionChunkLogSchema = z.object({
  agent_full_id: z.string(),
  agent_id: z.string(),
  agent_instance_hierarchy: z.string(),
  agent_remote: RemotePathSchema.nullable().meta({ omitempty: true }).optional(),
  continuation: LogReferenceSchema.nullable().meta({ omitempty: true }).optional(),
  created: z.number().int().min(0).max(18446744073709552e3),
  error: ErrorResponseErrorSchema.nullable().meta({ omitempty: true }).optional(),
  id: z.string(),
  messages: z.array(AgentCompletionsResponseStreamingLogReferenceSchema),
  messages_queued: z.boolean().nullable().meta({ omitempty: true }).optional(),
  object: AgentCompletionsResponseStreamingObjectSchema,
  upstream: AgentUpstreamSchema,
  usage: AgentCompletionsResponseUsageSchema.nullable().meta({ omitempty: true }).optional()
}).meta({ title: "agent.completions.response.streaming.AgentCompletionChunkLog" });
var AgentCompletionsResponseStreamingAssistantResponseChunkLogSchema = z.object({
  content: AgentCompletionsMessageRichContentLogSchema.nullable().meta({ omitempty: true }).optional(),
  created: z.number().int().min(0).max(18446744073709552e3),
  finish_reason: AgentCompletionsResponseFinishReasonSchema.nullable().optional(),
  index: z.number().int().min(0).max(18446744073709552e3),
  logprobs: LogReferenceSchema.nullable().meta({ omitempty: true }).optional(),
  model: z.string(),
  provider: z.string().nullable().meta({ omitempty: true }).optional(),
  reasoning: LogReferenceSchema.nullable().meta({ omitempty: true }).optional(),
  refusal: LogReferenceSchema.nullable().meta({ omitempty: true }).optional(),
  role: AgentCompletionsResponseAssistantRoleSchema,
  service_tier: z.string().nullable().meta({ omitempty: true }).optional(),
  system_fingerprint: z.string().nullable().meta({ omitempty: true }).optional(),
  tool_calls: z.array(LogReferenceSchema).nullable().meta({ omitempty: true }).optional(),
  upstream_id: z.string(),
  usage: AgentCompletionsResponseUpstreamUsageSchema.nullable().meta({ omitempty: true }).optional()
}).meta({ title: "agent.completions.response.streaming.AssistantResponseChunkLog" });
var AgentCompletionsResponseToolResponseLogSchema = z.object({
  content: AgentCompletionsMessageRichContentLogSchema,
  index: z.number().int().min(0).max(18446744073709552e3),
  metadata: AgentCompletionsMessageToolResponseMetadataSchema.nullable().meta({ omitempty: true }).optional(),
  role: AgentCompletionsResponseToolRoleSchema,
  tool_call_id: z.string()
}).meta({ title: "agent.completions.response.ToolResponseLog" });
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
var FunctionsExpressionInputValueLogSchema = z.union([LogReferenceSchema.meta({ "title": "LogReference", "variantTitle": "Reference" }), z.record(z.string(), LogReferenceSchema).meta({ "variantTitle": "Object" }), z.array(LogReferenceSchema).meta({ "variantTitle": "Array" })]).meta({ title: "functions.expression.InputValueLog" });
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
  depth: z.number().int().min(0).max(18446744073709552e3),
  input: FunctionsExpressionExpressionSchema,
  input_schema: FunctionsExpressionObjectInputSchemaSchema,
  max_branch_width: z.number().int().min(0).max(18446744073709552e3),
  max_leaf_width: z.number().int().min(0).max(18446744073709552e3),
  min_branch_width: z.number().int().min(0).max(18446744073709552e3),
  min_leaf_width: z.number().int().min(0).max(18446744073709552e3),
  name: z.string(),
  skip: FunctionsExpressionExpressionSchema.nullable().meta({ omitempty: true }).optional(),
  spec: z.string()
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
  depth: z.number().int().min(0).max(18446744073709552e3),
  input: FunctionsExpressionExpressionSchema,
  input_schema: FunctionsExpressionObjectInputSchemaSchema,
  max_branch_width: z.number().int().min(0).max(18446744073709552e3),
  max_leaf_width: z.number().int().min(0).max(18446744073709552e3),
  min_branch_width: z.number().int().min(0).max(18446744073709552e3),
  min_leaf_width: z.number().int().min(0).max(18446744073709552e3),
  name: z.string(),
  skip: FunctionsExpressionExpressionSchema.nullable().meta({ omitempty: true }).optional(),
  spec: z.string()
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
  depth: z.number().int().min(0).max(18446744073709552e3),
  input: FunctionsAlphaVectorExpressionVectorFunctionInputValueExpressionSchema,
  input_schema: FunctionsAlphaVectorExpressionVectorFunctionInputSchemaSchema,
  max_branch_width: z.number().int().min(0).max(18446744073709552e3),
  max_leaf_width: z.number().int().min(0).max(18446744073709552e3),
  min_branch_width: z.number().int().min(0).max(18446744073709552e3),
  min_leaf_width: z.number().int().min(0).max(18446744073709552e3),
  name: z.string(),
  skip: FunctionsExpressionExpressionSchema.nullable().meta({ omitempty: true }).optional(),
  spec: z.string()
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

// src/functions/executions/request/functionExecutionCreateParamsLog.ts
var FunctionsExecutionsRequestFunctionExecutionCreateParamsLogSchema = z.object({
  continuation: LogReferenceSchema.nullable().meta({ omitempty: true }).optional(),
  from_cache: z.boolean().nullable().meta({ omitempty: true }).optional(),
  function: FunctionsFullInlineFunctionOrRemoteCommitOptionalSchema,
  input: FunctionsExpressionInputValueLogSchema,
  invert: z.boolean().nullable().meta({ omitempty: true }).optional(),
  profile: FunctionsInlineProfileOrRemoteCommitOptionalSchema,
  provider: AgentCompletionsRequestProviderSchema.nullable().meta({ omitempty: true }).optional(),
  reasoning: FunctionsExecutionsRequestReasoningSchema.nullable().meta({ omitempty: true }).optional(),
  retry_token: z.string().nullable().meta({ omitempty: true }).optional(),
  seed: z.number().int().min(-9223372036854776e3).max(9223372036854776e3).nullable().meta({ omitempty: true }).optional(),
  split: z.boolean().nullable().meta({ omitempty: true }).optional(),
  strategy: FunctionsExecutionsRequestStrategySchema.nullable().meta({ omitempty: true }).optional(),
  stream: z.boolean().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "functions.executions.request.FunctionExecutionCreateParamsLog" });
var FunctionsExpressionTaskOutputSchema = z.union([z.number().min(-34028234663852886e22).max(34028234663852886e22).describe("A single scalar score.").meta({ "variantTitle": "Scalar" }), z.array(z.number().min(-34028234663852886e22).max(34028234663852886e22)).describe("A vector of scores.").meta({ "variantTitle": "Vector" }), z.array(z.array(z.number().min(-34028234663852886e22).max(34028234663852886e22))).describe("Multiple vectors of scores (from mapped tasks).").meta({ "variantTitle": "Vectors" }), z.object({
  error: JsonValueSchema
}).describe("An error occurred during execution.").meta({ "variantTitle": "Err" })]).describe("Owned task output variants.").meta({ title: "functions.expression.TaskOutput" });

// src/functions/executions/response/output.ts
var FunctionsExecutionsResponseOutputSchema = z.object({
  output: FunctionsExpressionTaskOutputSchema
}).describe("Wrapper for function execution output, distinguishing between\na null output value and a missing output.").meta({ title: "functions.executions.response.Output" });
var FunctionsExecutionsResponseStreamingObjectSchema = z.enum(["scalar.function.execution.chunk", "vector.function.execution.chunk"]).meta({ title: "functions.executions.response.streaming.Object" });
var FunctionsExecutionsResponseStreamingReasoningSummaryLogReferenceLogReferenceSchema = z.object({
  error: JsonValueSchema.meta({ omitempty: true }),
  path: z.string().meta({ omitempty: true }),
  type: LogReferenceTagSchema
}).meta({ title: "functions.executions.response.streaming.reasoning_summary_log_reference.LogReference" });
var FunctionsExecutionsResponseStreamingFunctionExecutionTaskLogReferenceLogReferenceSchema = z.object({
  index: z.number().int().min(0).max(18446744073709552e3),
  path: z.string().meta({ omitempty: true }),
  split_index: z.number().int().min(0).max(18446744073709552e3).nullable().meta({ omitempty: true }).optional(),
  swiss_pool_index: z.number().int().min(0).max(18446744073709552e3).nullable().meta({ omitempty: true }).optional(),
  swiss_round: z.number().int().min(0).max(18446744073709552e3).nullable().meta({ omitempty: true }).optional(),
  task_index: z.number().int().min(0).max(18446744073709552e3),
  task_path: z.array(z.number().int().min(0).max(18446744073709552e3)),
  type: LogReferenceTagSchema
}).meta({ title: "functions.executions.response.streaming.function_execution_task_log_reference.LogReference" });
var FunctionsExecutionsResponseStreamingVectorCompletionTaskLogReferenceLogReferenceSchema = z.object({
  error: JsonValueSchema.meta({ omitempty: true }),
  index: z.number().int().min(0).max(18446744073709552e3),
  path: z.string().meta({ omitempty: true }),
  task_index: z.number().int().min(0).max(18446744073709552e3),
  task_path: z.array(z.number().int().min(0).max(18446744073709552e3)),
  type: LogReferenceTagSchema
}).meta({ title: "functions.executions.response.streaming.vector_completion_task_log_reference.LogReference" });

// src/functions/executions/response/streaming/task_log_reference/logReference.ts
var FunctionsExecutionsResponseStreamingTaskLogReferenceLogReferenceSchema = z.union([FunctionsExecutionsResponseStreamingFunctionExecutionTaskLogReferenceLogReferenceSchema.meta({ "title": "functions.executions.response.streaming.function_execution_task_log_reference.LogReference", "variantTitle": "FunctionExecution" }), FunctionsExecutionsResponseStreamingVectorCompletionTaskLogReferenceLogReferenceSchema.meta({ "title": "functions.executions.response.streaming.vector_completion_task_log_reference.LogReference", "variantTitle": "VectorCompletion" })]).meta({ title: "functions.executions.response.streaming.task_log_reference.LogReference" });

// src/functions/executions/response/streaming/functionExecutionChunkLog.ts
var FunctionsExecutionsResponseStreamingFunctionExecutionChunkLogSchema = z.object({
  created: z.number().int().min(0).max(18446744073709552e3),
  error: ErrorResponseErrorSchema.nullable().meta({ omitempty: true }).optional(),
  function: RemotePathSchema.nullable().optional(),
  id: z.string(),
  object: FunctionsExecutionsResponseStreamingObjectSchema,
  output: FunctionsExecutionsResponseOutputSchema.nullable().meta({ omitempty: true }).optional(),
  profile: RemotePathSchema.nullable().optional(),
  reasoning: FunctionsExecutionsResponseStreamingReasoningSummaryLogReferenceLogReferenceSchema.nullable().describe("Appended at the end to match the legacy on-disk order: the\nold code inserted `reasoning` via `Map::insert` AFTER all\nthe shell fields, putting it at the tail of the object.").meta({ omitempty: true }).optional(),
  retry_token: LogReferenceSchema.nullable().meta({ omitempty: true }).optional(),
  tasks: z.array(FunctionsExecutionsResponseStreamingTaskLogReferenceLogReferenceSchema),
  tasks_errors: z.boolean().nullable().meta({ omitempty: true }).optional(),
  usage: AgentCompletionsResponseUsageSchema.nullable().meta({ omitempty: true }).optional()
}).meta({ title: "functions.executions.response.streaming.FunctionExecutionChunkLog" });
var FunctionsInventionsPromptsStepPromptTypeSchema = z.enum(["alpha.scalar.branch.function", "alpha.scalar.leaf.function", "alpha.vector.branch.function", "alpha.vector.leaf.function"]).describe("The type of invention state a prompt applies to.").meta({ title: "functions.inventions.prompts.StepPromptType" });

// src/functions/inventions/prompts/stepPromptExpression.ts
var FunctionsInventionsPromptsStepPromptExpressionSchema = z.object({
  type: z.array(FunctionsInventionsPromptsStepPromptTypeSchema),
  value: z.union([FunctionsExpressionExpressionSchema.describe("An expression (JMESPath or Starlark) to evaluate.").meta({ "title": "functions.expression.Expression", "variantTitle": "Expression" }), z.string().describe("A literal value.").meta({ "variantTitle": "Value" })]).describe('A value that can be either a literal or an expression.\n\nThis allows Function definitions to mix static values with dynamic\nexpressions. During compilation, expressions are evaluated while\nliteral values pass through unchanged.\n\n# Example\n\nLiteral value:\n```json\n"hello world"\n```\n\nJMESPath expression:\n```json\n{"$jmespath": "input.greeting"}\n```\n\nStarlark expression:\n```json\n{"$starlark": "input[\'greeting\']"}\n```')
}).describe("A prompt for a single invention step, applicable to one or more state types.").meta({ title: "functions.inventions.prompts.StepPromptExpression" });

// src/functions/inventions/prompts/inlinePrompt.ts
var FunctionsInventionsPromptsInlinePromptSchema = z.object({
  description_step: z.array(FunctionsInventionsPromptsStepPromptExpressionSchema),
  essay_step: z.array(FunctionsInventionsPromptsStepPromptExpressionSchema),
  essay_tasks_step: z.array(FunctionsInventionsPromptsStepPromptExpressionSchema),
  input_schema_step: z.array(FunctionsInventionsPromptsStepPromptExpressionSchema),
  tasks_step: z.array(FunctionsInventionsPromptsStepPromptExpressionSchema)
}).describe("Inline invention prompt configuration for all steps.").meta({ title: "functions.inventions.prompts.InlinePrompt" });

// src/functions/inventions/prompts/inlinePromptOrRemoteCommitOptional.ts
var FunctionsInventionsPromptsInlinePromptOrRemoteCommitOptionalSchema = z.union([FunctionsInventionsPromptsInlinePromptSchema.meta({ "title": "functions.inventions.prompts.InlinePrompt", "variantTitle": "Inline" }), RemotePathCommitOptionalSchema.meta({ "title": "RemotePathCommitOptional", "variantTitle": "Remote" })]).describe("A prompt specification that is either an inline prompt definition\nor a remote path reference.").meta({ title: "functions.inventions.prompts.InlinePromptOrRemoteCommitOptional" });
var RemoteSchema = z.union([z.literal("github").describe("GitHub repository.").meta({ "variantTitle": "Github" }), z.literal("filesystem").describe("Local filesystem.").meta({ "variantTitle": "Filesystem" }), z.literal("mock").describe("Mock (for testing).").meta({ "variantTitle": "Mock" })]).describe("The remote source where a function, profile, or agent is hosted.").meta({ title: "Remote" });

// src/functions/inventions/recursive/request/functionInventionRecursiveCreateParamsLog.ts
var FunctionsInventionsRecursiveRequestFunctionInventionRecursiveCreateParamsLogSchema = z.object({
  agent: AgentInlineAgentBaseWithFallbacksOrRemoteCommitOptionalSchema,
  continuation: LogReferenceSchema.nullable().meta({ omitempty: true }).optional(),
  max_step_retries: z.number().int().min(0).max(4294967295).nullable().meta({ omitempty: true }).optional(),
  overwrite: z.boolean().nullable().meta({ omitempty: true }).optional(),
  prompt: FunctionsInventionsPromptsInlinePromptOrRemoteCommitOptionalSchema,
  provider: AgentCompletionsRequestProviderSchema.nullable().meta({ omitempty: true }).optional(),
  remote: RemoteSchema,
  seed: z.number().int().min(-9223372036854776e3).max(9223372036854776e3).nullable().meta({ omitempty: true }).optional(),
  state: LogReferenceSchema,
  stream: z.boolean().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "functions.inventions.recursive.request.FunctionInventionRecursiveCreateParamsLog" });
var FunctionsInventionsRecursiveResponseStreamingObjectSchema = z.enum(["alpha.scalar.function.invention.recursive.chunk", "alpha.vector.function.invention.recursive.chunk"]).meta({ title: "functions.inventions.recursive.response.streaming.Object" });
var IndexedLogReferenceSchema = z.object({
  error: ErrorResponseErrorSchema.nullable().meta({ omitempty: true }).optional(),
  index: z.number().int().min(0).max(18446744073709552e3),
  path: z.string().nullable().meta({ omitempty: true }).optional(),
  type: LogReferenceTagSchema
}).describe("`LogReference` for log files keyed by an `index` \u2014 used by\nper-agent / per-invention completion wrappers that need to preserve\ntheir position within a parent collection (a vector completion's\nswarm-index, an invention's per-invention index, etc.).\n\nCarries an optional `error` so that wrappers around inner streams\nwhich errored before producing an ID-bearing chunk still surface\nwhat went wrong at that index. Exactly one of `path` (the resolved\nlog file) or `error` (the upstream failure) will be populated in\npractice \u2014 both `None` would mean the wrapper has nothing to say.").meta({ title: "IndexedLogReference" });

// src/functions/inventions/recursive/response/streaming/functionInventionRecursiveChunkLog.ts
var FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunkLogSchema = z.object({
  created: z.number().int().min(0).max(18446744073709552e3),
  id: z.string(),
  inventions: z.array(IndexedLogReferenceSchema),
  inventions_errors: z.boolean().nullable().meta({ omitempty: true }).optional(),
  object: FunctionsInventionsRecursiveResponseStreamingObjectSchema,
  usage: AgentCompletionsResponseUsageSchema.nullable().meta({ omitempty: true }).optional()
}).meta({ title: "functions.inventions.recursive.response.streaming.FunctionInventionRecursiveChunkLog" });
var FunctionsInventionsStateAlphaScalarBranchStateSchema = z.object({
  depth: z.number().int().min(0).max(18446744073709552e3),
  description: z.string().nullable().meta({ omitempty: true }).optional(),
  essay: z.string().nullable().meta({ omitempty: true }).optional(),
  essay_tasks: z.string().nullable().meta({ omitempty: true }).optional(),
  input_schema: FunctionsExpressionObjectInputSchemaSchema.nullable().meta({ omitempty: true }).optional(),
  max_branch_width: z.number().int().min(0).max(18446744073709552e3),
  max_leaf_width: z.number().int().min(0).max(18446744073709552e3),
  min_branch_width: z.number().int().min(0).max(18446744073709552e3),
  min_leaf_width: z.number().int().min(0).max(18446744073709552e3),
  name: z.string(),
  readme: z.string().nullable().meta({ omitempty: true }).optional(),
  spec: z.string(),
  tasks: z.array(FunctionsAlphaScalarBranchTaskExpressionSchema).nullable().meta({ omitempty: true }).optional(),
  tasks_length: z.number().int().min(0).max(18446744073709552e3).nullable().meta({ omitempty: true }).optional()
}).meta({ title: "functions.inventions.state.AlphaScalarBranchState" });
var FunctionsInventionsStateAlphaScalarLeafStateSchema = z.object({
  depth: z.number().int().min(0).max(18446744073709552e3),
  description: z.string().nullable().meta({ omitempty: true }).optional(),
  essay: z.string().nullable().meta({ omitempty: true }).optional(),
  essay_tasks: z.string().nullable().meta({ omitempty: true }).optional(),
  input_schema: FunctionsExpressionObjectInputSchemaSchema.nullable().meta({ omitempty: true }).optional(),
  max_branch_width: z.number().int().min(0).max(18446744073709552e3),
  max_leaf_width: z.number().int().min(0).max(18446744073709552e3),
  min_branch_width: z.number().int().min(0).max(18446744073709552e3),
  min_leaf_width: z.number().int().min(0).max(18446744073709552e3),
  name: z.string(),
  readme: z.string().nullable().meta({ omitempty: true }).optional(),
  spec: z.string(),
  tasks: z.array(FunctionsAlphaScalarLeafTaskExpressionSchema).nullable().meta({ omitempty: true }).optional(),
  tasks_length: z.number().int().min(0).max(18446744073709552e3).nullable().meta({ omitempty: true }).optional()
}).meta({ title: "functions.inventions.state.AlphaScalarLeafState" });
var FunctionsInventionsStateAlphaScalarStateSchema = z.object({
  depth: z.number().int().min(0).max(18446744073709552e3),
  input_schema: FunctionsExpressionObjectInputSchemaSchema.nullable().meta({ omitempty: true }).optional(),
  max_branch_width: z.number().int().min(0).max(18446744073709552e3),
  max_leaf_width: z.number().int().min(0).max(18446744073709552e3),
  min_branch_width: z.number().int().min(0).max(18446744073709552e3),
  min_leaf_width: z.number().int().min(0).max(18446744073709552e3),
  name: z.string(),
  spec: z.string()
}).meta({ title: "functions.inventions.state.AlphaScalarState" });
var FunctionsInventionsStateAlphaVectorBranchStateSchema = z.object({
  depth: z.number().int().min(0).max(18446744073709552e3),
  description: z.string().nullable().meta({ omitempty: true }).optional(),
  essay: z.string().nullable().meta({ omitempty: true }).optional(),
  essay_tasks: z.string().nullable().meta({ omitempty: true }).optional(),
  input_schema: FunctionsAlphaVectorExpressionVectorFunctionInputSchemaSchema.nullable().meta({ omitempty: true }).optional(),
  max_branch_width: z.number().int().min(0).max(18446744073709552e3),
  max_leaf_width: z.number().int().min(0).max(18446744073709552e3),
  min_branch_width: z.number().int().min(0).max(18446744073709552e3),
  min_leaf_width: z.number().int().min(0).max(18446744073709552e3),
  name: z.string(),
  readme: z.string().nullable().meta({ omitempty: true }).optional(),
  spec: z.string(),
  tasks: z.array(FunctionsAlphaVectorBranchTaskExpressionSchema).nullable().meta({ omitempty: true }).optional(),
  tasks_length: z.number().int().min(0).max(18446744073709552e3).nullable().meta({ omitempty: true }).optional()
}).meta({ title: "functions.inventions.state.AlphaVectorBranchState" });
var FunctionsInventionsStateAlphaVectorLeafStateSchema = z.object({
  depth: z.number().int().min(0).max(18446744073709552e3),
  description: z.string().nullable().meta({ omitempty: true }).optional(),
  essay: z.string().nullable().meta({ omitempty: true }).optional(),
  essay_tasks: z.string().nullable().meta({ omitempty: true }).optional(),
  input_schema: FunctionsAlphaVectorExpressionVectorFunctionInputSchemaSchema.nullable().meta({ omitempty: true }).optional(),
  max_branch_width: z.number().int().min(0).max(18446744073709552e3),
  max_leaf_width: z.number().int().min(0).max(18446744073709552e3),
  min_branch_width: z.number().int().min(0).max(18446744073709552e3),
  min_leaf_width: z.number().int().min(0).max(18446744073709552e3),
  name: z.string(),
  readme: z.string().nullable().meta({ omitempty: true }).optional(),
  spec: z.string(),
  tasks: z.array(FunctionsAlphaVectorLeafTaskExpressionSchema).nullable().meta({ omitempty: true }).optional(),
  tasks_length: z.number().int().min(0).max(18446744073709552e3).nullable().meta({ omitempty: true }).optional()
}).meta({ title: "functions.inventions.state.AlphaVectorLeafState" });
var FunctionsInventionsStateAlphaVectorStateSchema = z.object({
  depth: z.number().int().min(0).max(18446744073709552e3),
  input_schema: FunctionsAlphaVectorExpressionVectorFunctionInputSchemaSchema.nullable().meta({ omitempty: true }).optional(),
  max_branch_width: z.number().int().min(0).max(18446744073709552e3),
  max_leaf_width: z.number().int().min(0).max(18446744073709552e3),
  min_branch_width: z.number().int().min(0).max(18446744073709552e3),
  min_leaf_width: z.number().int().min(0).max(18446744073709552e3),
  name: z.string(),
  spec: z.string()
}).meta({ title: "functions.inventions.state.AlphaVectorState" });

// src/functions/inventions/state/paramsState.ts
var FunctionsInventionsStateParamsStateSchema = z.union([FunctionsInventionsStateAlphaScalarBranchStateSchema.and(z.object({
  type: z.literal("alpha.scalar.branch.function")
})).meta({ "variantTitle": "AlphaScalarBranch" }), FunctionsInventionsStateAlphaScalarLeafStateSchema.and(z.object({
  type: z.literal("alpha.scalar.leaf.function")
})).meta({ "variantTitle": "AlphaScalarLeaf" }), FunctionsInventionsStateAlphaVectorBranchStateSchema.and(z.object({
  type: z.literal("alpha.vector.branch.function")
})).meta({ "variantTitle": "AlphaVectorBranch" }), FunctionsInventionsStateAlphaVectorLeafStateSchema.and(z.object({
  type: z.literal("alpha.vector.leaf.function")
})).meta({ "variantTitle": "AlphaVectorLeaf" }), FunctionsInventionsStateAlphaScalarStateSchema.and(z.object({
  type: z.literal("alpha.scalar.function")
})).meta({ "variantTitle": "AlphaScalar" }), FunctionsInventionsStateAlphaVectorStateSchema.and(z.object({
  type: z.literal("alpha.vector.function")
})).meta({ "variantTitle": "AlphaVector" })]).meta({ title: "functions.inventions.state.ParamsState" });

// src/functions/inventions/state/paramsStateOrRemoteCommitOptional.ts
var FunctionsInventionsStateParamsStateOrRemoteCommitOptionalSchema = z.union([FunctionsInventionsStateParamsStateSchema.meta({ "title": "functions.inventions.state.ParamsState", "variantTitle": "ParamsState" }), RemotePathCommitOptionalSchema.meta({ "title": "RemotePathCommitOptional", "variantTitle": "Remote" })]).describe("A state specification that is either an inline ParamsState definition\nor a remote path reference.").meta({ title: "functions.inventions.state.ParamsStateOrRemoteCommitOptional" });

// src/functions/inventions/request/functionInventionCreateParams.ts
var FunctionsInventionsRequestFunctionInventionCreateParamsSchema = z.object({
  agent: AgentInlineAgentBaseWithFallbacksOrRemoteCommitOptionalSchema,
  continuation: z.string().nullable().describe("Continuation from a previous completion, as a base64-encoded string.").meta({ omitempty: true }).optional(),
  max_step_retries: z.number().int().min(0).max(4294967295).nullable().describe("Maximum number of retries per invention step.\nEach step is one agent completion (which itself may loop internally\nvia tool calls). If the step's validation still fails after the\nagent loop ends, the step is retried up to this many times.\nDefaults to 3 if not specified.").meta({ omitempty: true }).optional(),
  overwrite: z.boolean().nullable().meta({ omitempty: true }).optional(),
  prompt: FunctionsInventionsPromptsInlinePromptOrRemoteCommitOptionalSchema,
  provider: AgentCompletionsRequestProviderSchema.nullable().meta({ omitempty: true }).optional(),
  remote: RemoteSchema.nullable().meta({ omitempty: true }).optional(),
  seed: z.number().int().min(-9223372036854776e3).max(9223372036854776e3).nullable().meta({ omitempty: true }).optional(),
  state: FunctionsInventionsStateParamsStateOrRemoteCommitOptionalSchema,
  stream: z.boolean().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "functions.inventions.request.FunctionInventionCreateParams" });
var FunctionsAlphaScalarRemoteFunctionSchema = z.union([z.object({
  description: z.string(),
  input_schema: FunctionsExpressionObjectInputSchemaSchema,
  tasks: z.array(FunctionsAlphaScalarBranchTaskExpressionSchema),
  type: z.literal("alpha.scalar.branch.function")
}).meta({ "variantTitle": "Branch" }), z.object({
  description: z.string(),
  input_schema: FunctionsExpressionObjectInputSchemaSchema,
  tasks: z.array(FunctionsAlphaScalarLeafTaskExpressionSchema),
  type: z.literal("alpha.scalar.leaf.function")
}).meta({ "variantTitle": "Leaf" })]).meta({ title: "functions.alpha_scalar.RemoteFunction" });
var FunctionsAlphaVectorRemoteFunctionSchema = z.union([z.object({
  description: z.string(),
  input_schema: FunctionsAlphaVectorExpressionVectorFunctionInputSchemaSchema,
  tasks: z.array(FunctionsAlphaVectorBranchTaskExpressionSchema),
  type: z.literal("alpha.vector.branch.function")
}).meta({ "variantTitle": "Branch" }), z.object({
  description: z.string(),
  input_schema: FunctionsAlphaVectorExpressionVectorFunctionInputSchemaSchema,
  tasks: z.array(FunctionsAlphaVectorLeafTaskExpressionSchema),
  type: z.literal("alpha.vector.leaf.function")
}).meta({ "variantTitle": "Leaf" })]).meta({ title: "functions.alpha_vector.RemoteFunction" });

// src/functions/alphaRemoteFunction.ts
var FunctionsAlphaRemoteFunctionSchema = z.union([FunctionsAlphaScalarRemoteFunctionSchema.meta({ "title": "functions.alpha_scalar.RemoteFunction", "variantTitle": "Scalar" }), FunctionsAlphaVectorRemoteFunctionSchema.meta({ "title": "functions.alpha_vector.RemoteFunction", "variantTitle": "Vector" })]).meta({ title: "functions.AlphaRemoteFunction" });
var FunctionsRemoteFunctionSchema = z.union([z.object({
  description: z.string().describe("Human-readable description of what the function does."),
  input_schema: FunctionsExpressionInputSchemaSchema.describe("JSON Schema defining the expected input structure."),
  tasks: z.array(FunctionsTaskExpressionSchema).describe("The list of tasks to execute. Tasks with a `map` expression are\nexpanded into multiple instances. Each instance is compiled with\n`map` set to the current integer index.\nReceives: `input`, `map` (if mapped)."),
  type: z.literal("scalar.function")
}).describe("Produces a single score in [0, 1].").meta({ "variantTitle": "Scalar" }), z.object({
  description: z.string().describe("Human-readable description of what the function does."),
  input_merge: FunctionsExpressionExpressionSchema.describe("Expression transforming an array of inputs computed by `input_split`\ninto a single Input object for the Function.\nReceives: `input` (as an array)."),
  input_schema: FunctionsExpressionInputSchemaSchema.describe("JSON Schema defining the expected input structure."),
  input_split: FunctionsExpressionExpressionSchema.describe("Expression transforming input into an input array of the output_length\nWhen the Function is executed with any input from the array,\nThe output_length should be 1.\nReceives: `input`."),
  output_length: FunctionsExpressionExpressionSchema.describe("Expression computing the expected output vector length for task outputs.\nReceives: `input`."),
  tasks: z.array(FunctionsTaskExpressionSchema).describe("The list of tasks to execute. Tasks with a `map` expression are\nexpanded into multiple instances. Each instance is compiled with\n`map` set to the current integer index.\nReceives: `input`, `map` (if mapped)."),
  type: z.literal("vector.function")
}).describe("Produces a vector of scores that sums to 1.").meta({ "variantTitle": "Vector" })]).describe("A remote function with full metadata.\n\nRemote functions are stored as `function.json` in repositories and\nreferenced by `remote/owner/repository`. They include documentation fields\nthat inline functions lack.").meta({ title: "functions.RemoteFunction" });

// src/functions/fullRemoteFunction.ts
var FunctionsFullRemoteFunctionSchema = z.union([FunctionsAlphaRemoteFunctionSchema.meta({ "title": "functions.AlphaRemoteFunction", "variantTitle": "Alpha" }), FunctionsRemoteFunctionSchema.meta({ "title": "functions.RemoteFunction", "variantTitle": "Standard" })]).meta({ title: "functions.FullRemoteFunction" });
var FunctionsInventionsResponseStreamingObjectSchema = z.enum(["alpha.scalar.function.invention.chunk", "alpha.vector.function.invention.chunk"]).meta({ title: "functions.inventions.response.streaming.Object" });
var FunctionsInventionsStateStateSchema = z.union([FunctionsInventionsStateAlphaScalarBranchStateSchema.and(z.object({
  type: z.literal("alpha.scalar.branch.function")
})).meta({ "variantTitle": "AlphaScalarBranch" }), FunctionsInventionsStateAlphaScalarLeafStateSchema.and(z.object({
  type: z.literal("alpha.scalar.leaf.function")
})).meta({ "variantTitle": "AlphaScalarLeaf" }), FunctionsInventionsStateAlphaVectorBranchStateSchema.and(z.object({
  type: z.literal("alpha.vector.branch.function")
})).meta({ "variantTitle": "AlphaVectorBranch" }), FunctionsInventionsStateAlphaVectorLeafStateSchema.and(z.object({
  type: z.literal("alpha.vector.leaf.function")
})).meta({ "variantTitle": "AlphaVectorLeaf" })]).meta({ title: "functions.inventions.state.State" });

// src/functions/inventions/response/streaming/functionInventionChunkLog.ts
var FunctionsInventionsResponseStreamingFunctionInventionChunkLogSchema = z.object({
  completions: z.array(IndexedLogReferenceSchema),
  created: z.number().int().min(0).max(18446744073709552e3),
  error: ErrorResponseErrorSchema.nullable().meta({ omitempty: true }).optional(),
  function: FunctionsFullRemoteFunctionSchema.nullable().meta({ omitempty: true }).optional(),
  id: z.string(),
  object: FunctionsInventionsResponseStreamingObjectSchema,
  path: RemotePathSchema.nullable().meta({ omitempty: true }).optional(),
  state: FunctionsInventionsStateStateSchema.nullable().meta({ omitempty: true }).optional(),
  usage: AgentCompletionsResponseUsageSchema.nullable().meta({ omitempty: true }).optional()
}).meta({ title: "functions.inventions.response.streaming.FunctionInventionChunkLog" });
var SwarmInlineSwarmBaseOrRemoteCommitOptionalSchema = z.union([SwarmInlineSwarmBaseSchema.meta({ "title": "swarm.InlineSwarmBase", "variantTitle": "SwarmBase" }), RemotePathCommitOptionalSchema.meta({ "title": "RemotePathCommitOptional", "variantTitle": "Remote" })]).describe("Like [`InlineSwarmBaseOrRemote`] but with optional commit.\nUsed in request types where commit resolution happens server-side.").meta({ title: "swarm.InlineSwarmBaseOrRemoteCommitOptional" });

// src/vector/completions/request/vectorCompletionCreateParams.ts
var VectorCompletionsRequestVectorCompletionCreateParamsSchema = z.object({
  continuation: z.string().nullable().describe("Continuation from a previous completion, as a base64-encoded string.").meta({ omitempty: true }).optional(),
  from_cache: z.boolean().nullable().describe("If true, uses cached votes when available.").meta({ omitempty: true }).optional(),
  messages: z.array(AgentCompletionsMessageMessageSchema).describe("The conversation messages (the prompt)."),
  provider: AgentCompletionsRequestProviderSchema.nullable().describe("Provider routing preferences.").meta({ omitempty: true }).optional(),
  responses: z.array(AgentCompletionsMessageRichContentSchema).describe("The possible responses the LLMs can vote for."),
  retry: z.string().nullable().describe("If present, reuses votes from a previous request with this ID.").meta({ omitempty: true }).optional(),
  seed: z.number().int().min(-9223372036854776e3).max(9223372036854776e3).nullable().describe("Random seed for deterministic results.").meta({ omitempty: true }).optional(),
  stream: z.boolean().nullable().describe("Whether to stream the response.").meta({ omitempty: true }).optional(),
  swarm: SwarmInlineSwarmBaseOrRemoteCommitOptionalSchema.describe("The Swarm of agents to use.")
}).describe("Parameters for creating a vector completion.\n\nVector completions run multiple agent completions (one per LLM in the\nswarm), force each to vote for one of the predefined responses, and\ncombine votes using the provided profile weights to produce final scores.").meta({ title: "vector.completions.request.VectorCompletionCreateParams" });
var VectorCompletionsResponseStreamingObjectSchema = z.literal("vector.completion.chunk").describe("A streaming vector completion chunk.").meta({ title: "vector.completions.response.streaming.Object" });
var VectorCompletionsResponseVoteSchema = z.object({
  agent_full_id: z.string().describe("WF-level id of the agent that produced this vote \u2014 concatenation\nof the primary agent's id with all fallback ids (see\n`InlineAgentWithFallbacks::full_id`). Same for every slot in the\nsame WF request and deterministic across api processes."),
  agent_id: z.string().describe("Leaf agent id of the slot that produced this vote (matches the\n`agent_id` on the corresponding\n[`super::super::super::super::agent::completions::response::unary::AgentCompletion`]).\nWhen fallbacks fired, this is the fallback's id rather than the\nprimary's."),
  flat_swarm_index: z.number().int().min(0).max(18446744073709552e3).describe("Flattened index accounting for agent counts in the swarm."),
  from_cache: z.boolean().nullable().describe("If true, this vote was retrieved from cache rather than generated fresh.").meta({ omitempty: true }).optional(),
  prompt_id: z.string().describe("Content hash of the request messages (for caching/deduplication)."),
  responses_ids: z.array(z.string()).describe("Content hashes of each response option in the request."),
  retry: z.boolean().nullable().describe("If true, this vote was reused from a previous request via the `retry`\nparameter. All fields reflect the original request's values.").meta({ omitempty: true }).optional(),
  swarm_index: z.number().int().min(0).max(18446744073709552e3).describe("Index of the agent configuration within the swarm."),
  vote: z.array(z.number().min(-34028234663852886e22).max(34028234663852886e22)).describe("The vote distribution. Each index corresponds to a response from the\nrequest. Typically one element is 1.0 (selected) and the rest are 0.0."),
  weight: z.number().min(-34028234663852886e22).max(34028234663852886e22).describe("The weight applied to this vote when computing final scores.")
}).describe("A single LLM's vote in a vector completion.\n\nEach LLM in the swarm produces a vote indicating which response(s) it\nselected. Votes are weighted according to the profile and combined to\nproduce the final scores.\n\n# Vote Format\n\nThe `vote` field is a vector of decimals corresponding to the responses\nin the request. Typically one element is 1.0 and the rest are 0.0 (discrete\nselection), but when `top_logprobs` is used, votes may be probability\ndistributions.").meta({ title: "vector.completions.response.Vote" });

// src/vector/completions/response/streaming/vectorCompletionChunkLog.ts
var VectorCompletionsResponseStreamingVectorCompletionChunkLogSchema = z.object({
  completions: z.array(IndexedLogReferenceSchema),
  created: z.number().int().min(0).max(18446744073709552e3),
  id: z.string(),
  object: VectorCompletionsResponseStreamingObjectSchema,
  scores: z.array(z.number().min(-34028234663852886e22).max(34028234663852886e22)),
  swarm: z.string(),
  usage: AgentCompletionsResponseUsageSchema.nullable().meta({ omitempty: true }).optional(),
  votes: z.array(VectorCompletionsResponseVoteSchema),
  weights: z.array(z.number().min(-34028234663852886e22).max(34028234663852886e22))
}).meta({ title: "vector.completions.response.streaming.VectorCompletionChunkLog" });

// src/cli/command/agents/read/id/response.ts
var CliCommandAgentsReadIdResponseSchema = z.union([z.object({
  type: z.literal("agents_completions_response"),
  value: AgentCompletionsResponseStreamingAgentCompletionChunkLogSchema
}).meta({ "variantTitle": "AgentsCompletionsResponse" }), z.object({
  type: z.literal("agents_completions_request"),
  value: AgentCompletionsRequestAgentCompletionCreateParamsLogSchema
}).meta({ "variantTitle": "AgentsCompletionsRequest" }), z.object({
  type: z.literal("agents_completions_response_messages_assistant"),
  value: AgentCompletionsResponseStreamingAssistantResponseChunkLogSchema
}).meta({ "variantTitle": "AgentsCompletionsResponseMessagesAssistant" }), z.object({
  type: z.literal("agents_completions_response_messages_tool"),
  value: AgentCompletionsResponseToolResponseLogSchema
}).meta({ "variantTitle": "AgentsCompletionsResponseMessagesTool" }), z.object({
  type: z.literal("agents_completions_request_messages"),
  value: AgentCompletionsMessageMessageLogSchema
}).meta({ "variantTitle": "AgentsCompletionsRequestMessages" }), z.object({
  type: z.literal("agents_completions_response_messages_assistant_logprobs"),
  value: AgentCompletionsResponseLogprobsSchema
}).meta({ "variantTitle": "AgentsCompletionsResponseMessagesAssistantLogprobs" }), z.object({
  type: z.literal("agents_completions_response_messages_assistant_tool_calls"),
  value: AgentCompletionsMessageAssistantToolCallDeltaSchema
}).meta({ "variantTitle": "AgentsCompletionsResponseMessagesAssistantToolCalls" }), z.object({
  type: z.literal("agents_completions_request_messages_assistant_tool_calls"),
  value: AgentCompletionsMessageAssistantToolCallSchema
}).meta({ "variantTitle": "AgentsCompletionsRequestMessagesAssistantToolCalls" }), z.object({
  type: z.literal("vector_completions_response"),
  value: VectorCompletionsResponseStreamingVectorCompletionChunkLogSchema
}).meta({ "variantTitle": "VectorCompletionsResponse" }), z.object({
  type: z.literal("vector_completions_request"),
  value: VectorCompletionsRequestVectorCompletionCreateParamsSchema
}).meta({ "variantTitle": "VectorCompletionsRequest" }), z.object({
  type: z.literal("functions_executions_response"),
  value: FunctionsExecutionsResponseStreamingFunctionExecutionChunkLogSchema
}).meta({ "variantTitle": "FunctionsExecutionsResponse" }), z.object({
  type: z.literal("functions_executions_request"),
  value: FunctionsExecutionsRequestFunctionExecutionCreateParamsLogSchema
}).meta({ "variantTitle": "FunctionsExecutionsRequest" }), z.object({
  type: z.literal("functions_inventions_response"),
  value: FunctionsInventionsResponseStreamingFunctionInventionChunkLogSchema
}).meta({ "variantTitle": "FunctionsInventionsResponse" }), z.object({
  type: z.literal("functions_inventions_request"),
  value: FunctionsInventionsRequestFunctionInventionCreateParamsSchema
}).meta({ "variantTitle": "FunctionsInventionsRequest" }), z.object({
  type: z.literal("functions_inventions_recursive_response"),
  value: FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunkLogSchema
}).meta({ "variantTitle": "FunctionsInventionsRecursiveResponse" }), z.object({
  type: z.literal("functions_inventions_recursive_request"),
  value: FunctionsInventionsRecursiveRequestFunctionInventionRecursiveCreateParamsLogSchema
}).meta({ "variantTitle": "FunctionsInventionsRecursiveRequest" }), z.object({
  type: z.literal("text"),
  value: z.string()
}).meta({ "variantTitle": "Text" }), z.object({
  type: z.literal("image"),
  value: AgentCompletionsMessageImageUrlSchema
}).meta({ "variantTitle": "Image" }), z.object({
  type: z.literal("audio"),
  value: AgentCompletionsMessageInputAudioSchema
}).meta({ "variantTitle": "Audio" }), z.object({
  type: z.literal("video"),
  value: AgentCompletionsMessageVideoUrlSchema
}).meta({ "variantTitle": "Video" }), z.object({
  type: z.literal("file"),
  value: AgentCompletionsMessageFileSchema
}).meta({ "variantTitle": "File" })]).meta({ title: "cli.command.agents.read.id.Response" });

// src/viewer/command/agents/read/id.ts
async function agentsReadIdExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/read/id" }), z.union([CliErrorSchema, CliCommandAgentsReadIdResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents read id: cli produced no output before the end marker");
  }
  return first;
}
async function agentsReadIdExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/read/id" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents read id: cli produced no output before the end marker");
  }
  return first;
}
async function agentsReadIdRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/read/id/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents read id request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsReadIdRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/read/id/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents read id request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsReadIdResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/read/id/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents read id response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsReadIdResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/read/id/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents read id response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandAgentsReadPendingResponseItemSchema = z.object({
  agent_id: z.string(),
  items: z.array(CliCommandAgentsReadAllResponseQueueItemSchema)
}).meta({ title: "cli.command.agents.read.pending.ResponseItem" });

// src/viewer/command/agents/read/pending.ts
function agentsReadPendingExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/read/pending" }), z.union([CliErrorSchema, CliCommandAgentsReadPendingResponseItemSchema]));
}
function agentsReadPendingExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/read/pending" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function agentsReadPendingRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/read/pending/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents read pending request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsReadPendingRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/read/pending/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents read pending request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsReadPendingResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/read/pending/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents read pending response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsReadPendingResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/read/pending/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents read pending response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandAgentsReadSubscribeResponseItemSchema = z.union([z.object({
  agent_id: z.string(),
  items: z.array(CliCommandAgentsReadAllResponseQueueItemSchema)
}).meta({ "variantTitle": "Items" }), z.object({
  agent_id: z.string()
}).meta({ "variantTitle": "Inactive" })]).meta({ title: "cli.command.agents.read.subscribe.ResponseItem" });

// src/viewer/command/agents/read/subscribe.ts
function agentsReadSubscribeExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/read/subscribe" }), z.union([CliErrorSchema, CliCommandAgentsReadSubscribeResponseItemSchema]));
}
function agentsReadSubscribeExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/read/subscribe" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function agentsReadSubscribeRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/read/subscribe/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents read subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsReadSubscribeRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/read/subscribe/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents read subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsReadSubscribeResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/read/subscribe/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents read subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsReadSubscribeResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/read/subscribe/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents read subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandAgentsSpawnResponseItemSchema = z.union([AgentCompletionsResponseStreamingAgentCompletionChunkSchema.meta({ "title": "agent.completions.response.streaming.AgentCompletionChunk", "variantTitle": "Chunk" }), z.string().meta({ "variantTitle": "Id" })]).meta({ title: "cli.command.agents.spawn.ResponseItem" });

// src/viewer/command/agents/spawn.ts
function agentsSpawnExecuteStreaming(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, dangerous_advanced: { ...request.dangerous_advanced ?? {}, stream: true }, path_type: "agents/spawn" }), z.union([CliErrorSchema, CliCommandAgentsSpawnResponseItemSchema]));
}
function agentsSpawnExecuteStreamingJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, dangerous_advanced: { ...request.dangerous_advanced ?? {}, stream: true }, path_type: "agents/spawn" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function agentsSpawnExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, dangerous_advanced: request.dangerous_advanced ? { ...request.dangerous_advanced, stream: void 0 } : request.dangerous_advanced, path_type: "agents/spawn" }), z.union([CliErrorSchema, z.string()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents spawn: cli produced no output before the end marker");
  }
  return first;
}
async function agentsSpawnExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, dangerous_advanced: request.dangerous_advanced ? { ...request.dangerous_advanced, stream: void 0 } : request.dangerous_advanced, path_type: "agents/spawn" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents spawn: cli produced no output before the end marker");
  }
  return first;
}
async function agentsSpawnRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/spawn/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents spawn request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsSpawnRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/spawn/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents spawn request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsSpawnResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/spawn/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents spawn response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsSpawnResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/spawn/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents spawn response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandAgentsTagsAddResponseSchema = z.object({
  agent_full_id: z.string(),
  name: z.string(),
  parent_agent_instance_hierarchy: z.string().describe("Resolved parent scope \u2014 filled in with the cli's own\n`Config.agent_instance_hierarchy` when the caller did not\nsupply one.")
}).meta({ title: "cli.command.agents.tags.add.Response" });

// src/viewer/command/agents/tags/add.ts
async function agentsTagsAddExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/tags/add" }), z.union([CliErrorSchema, CliCommandAgentsTagsAddResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tags add: cli produced no output before the end marker");
  }
  return first;
}
async function agentsTagsAddExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/tags/add" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tags add: cli produced no output before the end marker");
  }
  return first;
}
async function agentsTagsAddRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/tags/add/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tags add request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsTagsAddRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/tags/add/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tags add request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsTagsAddResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/tags/add/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tags add response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsTagsAddResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/tags/add/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tags add response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsTagsLookupRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/tags/lookup/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tags lookup request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsTagsLookupRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/tags/lookup/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tags lookup request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsTagsLookupResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/tags/lookup/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tags lookup response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsTagsLookupResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/tags/lookup/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tags lookup response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandAgentsTasksListResponseItemSchema = z.object({
  agent_instance_hierarchy: z.string(),
  command: z.array(z.string()),
  created_at: z.number().int().min(-9223372036854776e3).max(9223372036854776e3),
  description: z.string(),
  id: z.string().describe('Stable identifier \u2014 `"{name}-{db_id}"` where `name` is the\n`--name` passed to `agents tasks schedule` and `db_id` is\nthe row id from `schedules`. Same shape `agents tasks run`\ntags each emitted item with.'),
  interval: z.string().nullable().describe('`None` for a oneshot; `Some("30s" / "1h" / "1d12h" / \u2026)`\nfor a recurring schedule, formatted as humantime so the\nlist output reads naturally without a unit-conversion\nstep at the consumer. The CLI parser accepts the same\nshape on `agents tasks schedule --interval`.').meta({ omitempty: true }).optional(),
  last_ran_at: z.number().int().min(-9223372036854776e3).max(9223372036854776e3).nullable().describe("Unix seconds of the most recent invocation. `None` until\nthe runner has fired this schedule at least once.").meta({ omitempty: true }).optional()
}).describe("One schedule row.").meta({ title: "cli.command.agents.tasks.list.ResponseItem" });

// src/viewer/command/agents/tasks/list.ts
function agentsTasksListExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/tasks/list" }), z.union([CliErrorSchema, CliCommandAgentsTasksListResponseItemSchema]));
}
function agentsTasksListExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/tasks/list" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function agentsTasksListRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/tasks/list/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tasks list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsTasksListRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/tasks/list/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tasks list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsTasksListResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/tasks/list/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tasks list response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsTasksListResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/tasks/list/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tasks list response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandAgentsTasksRunResponseItemSchema = z.object({
  id: z.string(),
  value: JsonValueSchema
}).describe("One output item from one fired schedule's in-process stream.\n`id` is the source schedule's stable identifier, formatted\n`\"{name}-{db_id}\"` so it's both human-readable (the user-\nsupplied `--name` from `schedule`) and globally unique (the\nrow id from `schedules`). `value` is the typed root\n[`crate::cli::command::ResponseItem`] emitted by the scheduled\ncli leaf \u2014 boxed because the root union transitively contains\n*this* variant (`agents \u2192 tasks \u2192 run`), and boxing is what\nmakes the recursion sized.\n\nThe `value` field's JSON schema is opaqued to `serde_json::Value`\n(renders as bare `{}` aka JsonValue) so the published schema\ndoesn't inline the entire root union \u2014 that's the TS7056 blowup\nthe root and tier aggregates dodge by being `json_schema_ignore`.\nDownstream SDKs see `value: JsonValue` on the typed `execute`\npath; consumers that want to peer inside parse it case-by-case.").meta({ title: "cli.command.agents.tasks.run.ResponseItem" });

// src/viewer/command/agents/tasks/run.ts
function agentsTasksRunExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/tasks/run" }), z.union([CliErrorSchema, CliCommandAgentsTasksRunResponseItemSchema]));
}
function agentsTasksRunExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/tasks/run" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function agentsTasksRunRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/tasks/run/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tasks run request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsTasksRunRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/tasks/run/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tasks run request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsTasksRunResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/tasks/run/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tasks run response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsTasksRunResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/tasks/run/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tasks run response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandAgentsTasksScheduleResponseSchema = z.object({
  id: z.string()
}).describe('`id` is the stable identifier `"{name}-{db_id}"` \u2014 the\nuser-supplied `--name` joined to the row id from\n`tasks.sqlite`\'s `schedules` table. Same shape `list` and\n`run` use to identify rows on the wire.').meta({ title: "cli.command.agents.tasks.schedule.Response" });

// src/viewer/command/agents/tasks/schedule.ts
async function agentsTasksScheduleExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/tasks/schedule" }), z.union([CliErrorSchema, CliCommandAgentsTasksScheduleResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tasks schedule: cli produced no output before the end marker");
  }
  return first;
}
async function agentsTasksScheduleExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/tasks/schedule" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tasks schedule: cli produced no output before the end marker");
  }
  return first;
}
async function agentsTasksScheduleRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/tasks/schedule/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tasks schedule request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsTasksScheduleRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/tasks/schedule/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tasks schedule request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsTasksScheduleResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "agents/tasks/schedule/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tasks schedule response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function agentsTasksScheduleResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "agents/tasks/schedule/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("agents tasks schedule response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandOkSchema = z.literal("Ok").describe('Success-only response shape. Wire form is the bare string `"Ok"` \u2014\na single-variant enum gives us a typed sentinel that serializes and\ndeserializes through serde as the static string. Used as `Response`\non every cli leaf whose only success signal is "it worked."').meta({ title: "cli.command.Ok" });

// src/viewer/command/config/agents/favorites/add.ts
async function configAgentsFavoritesAddExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/agents/favorites/add" }), z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents favorites add: cli produced no output before the end marker");
  }
  return first;
}
async function configAgentsFavoritesAddExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/agents/favorites/add" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents favorites add: cli produced no output before the end marker");
  }
  return first;
}
async function configAgentsFavoritesAddRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/agents/favorites/add/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents favorites add request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configAgentsFavoritesAddRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/agents/favorites/add/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents favorites add request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configAgentsFavoritesAddResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/agents/favorites/add/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents favorites add response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configAgentsFavoritesAddResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/agents/favorites/add/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents favorites add response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configAgentsFavoritesDelExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/agents/favorites/del" }), z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents favorites del: cli produced no output before the end marker");
  }
  return first;
}
async function configAgentsFavoritesDelExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/agents/favorites/del" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents favorites del: cli produced no output before the end marker");
  }
  return first;
}
async function configAgentsFavoritesDelRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/agents/favorites/del/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents favorites del request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configAgentsFavoritesDelRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/agents/favorites/del/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents favorites del request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configAgentsFavoritesDelResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/agents/favorites/del/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents favorites del response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configAgentsFavoritesDelResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/agents/favorites/del/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents favorites del response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configAgentsFavoritesEditExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/agents/favorites/edit" }), z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents favorites edit: cli produced no output before the end marker");
  }
  return first;
}
async function configAgentsFavoritesEditExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/agents/favorites/edit" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents favorites edit: cli produced no output before the end marker");
  }
  return first;
}
async function configAgentsFavoritesEditRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/agents/favorites/edit/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents favorites edit request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configAgentsFavoritesEditRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/agents/favorites/edit/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents favorites edit request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configAgentsFavoritesEditResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/agents/favorites/edit/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents favorites edit response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configAgentsFavoritesEditResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/agents/favorites/edit/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents favorites edit response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandConfigAgentsFavoritesGetResponseItemSchema = z.union([z.object({
  commit: z.string().nullable().optional(),
  name: z.string(),
  note: z.string(),
  owner: z.string(),
  remote: z.literal("github"),
  repository: z.string()
}).meta({ "variantTitle": "Github" }), z.object({
  commit: z.string().nullable().optional(),
  name: z.string(),
  note: z.string(),
  owner: z.string(),
  remote: z.literal("filesystem"),
  repository: z.string()
}).meta({ "variantTitle": "Filesystem" }), z.object({
  name: z.string(),
  note: z.string(),
  remote: z.literal("mock")
}).meta({ "variantTitle": "Mock" })]).meta({ title: "cli.command.config.agents.favorites.get.ResponseItem" });

// src/viewer/command/config/agents/favorites/get.ts
function configAgentsFavoritesGetExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/agents/favorites/get" }), z.union([CliErrorSchema, CliCommandConfigAgentsFavoritesGetResponseItemSchema]));
}
function configAgentsFavoritesGetExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/agents/favorites/get" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function configAgentsFavoritesGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/agents/favorites/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents favorites get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configAgentsFavoritesGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/agents/favorites/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents favorites get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configAgentsFavoritesGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/agents/favorites/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents favorites get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configAgentsFavoritesGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/agents/favorites/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents favorites get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandConfigAgentsGetResponseSchema = z.object({
  favorites: z.array(CliCommandConfigAgentsFavoritesGetResponseItemSchema).nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.config.agents.get.Response" });

// src/viewer/command/config/agents/get.ts
async function configAgentsGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/agents/get" }), z.union([CliErrorSchema, CliCommandConfigAgentsGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents get: cli produced no output before the end marker");
  }
  return first;
}
async function configAgentsGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/agents/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents get: cli produced no output before the end marker");
  }
  return first;
}
async function configAgentsGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/agents/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configAgentsGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/agents/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configAgentsGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/agents/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configAgentsGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/agents/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config agents get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsFavoritesAddExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/functions/favorites/add" }), z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions favorites add: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsFavoritesAddExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/functions/favorites/add" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions favorites add: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsFavoritesAddRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/favorites/add/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions favorites add request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsFavoritesAddRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/favorites/add/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions favorites add request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsFavoritesAddResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/favorites/add/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions favorites add response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsFavoritesAddResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/favorites/add/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions favorites add response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsFavoritesDelExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/functions/favorites/del" }), z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions favorites del: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsFavoritesDelExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/functions/favorites/del" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions favorites del: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsFavoritesDelRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/favorites/del/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions favorites del request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsFavoritesDelRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/favorites/del/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions favorites del request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsFavoritesDelResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/favorites/del/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions favorites del response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsFavoritesDelResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/favorites/del/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions favorites del response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsFavoritesEditExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/functions/favorites/edit" }), z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions favorites edit: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsFavoritesEditExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/functions/favorites/edit" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions favorites edit: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsFavoritesEditRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/favorites/edit/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions favorites edit request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsFavoritesEditRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/favorites/edit/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions favorites edit request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsFavoritesEditResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/favorites/edit/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions favorites edit response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsFavoritesEditResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/favorites/edit/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions favorites edit response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandConfigFunctionsFavoritesGetResponseItemSchema = z.union([z.object({
  commit: z.string().nullable().optional(),
  name: z.string(),
  note: z.string(),
  owner: z.string(),
  remote: z.literal("github"),
  repository: z.string()
}).meta({ "variantTitle": "Github" }), z.object({
  commit: z.string().nullable().optional(),
  name: z.string(),
  note: z.string(),
  owner: z.string(),
  remote: z.literal("filesystem"),
  repository: z.string()
}).meta({ "variantTitle": "Filesystem" }), z.object({
  name: z.string(),
  note: z.string(),
  remote: z.literal("mock")
}).meta({ "variantTitle": "Mock" })]).meta({ title: "cli.command.config.functions.favorites.get.ResponseItem" });

// src/viewer/command/config/functions/favorites/get.ts
function configFunctionsFavoritesGetExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/favorites/get" }), z.union([CliErrorSchema, CliCommandConfigFunctionsFavoritesGetResponseItemSchema]));
}
function configFunctionsFavoritesGetExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/favorites/get" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function configFunctionsFavoritesGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/favorites/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions favorites get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsFavoritesGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/favorites/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions favorites get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsFavoritesGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/favorites/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions favorites get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsFavoritesGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/favorites/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions favorites get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandConfigFunctionsGetResponseSchema = z.object({
  favorites: z.array(CliCommandConfigFunctionsFavoritesGetResponseItemSchema).nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.config.functions.get.Response" });

// src/viewer/command/config/functions/get.ts
async function configFunctionsGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/get" }), z.union([CliErrorSchema, CliCommandConfigFunctionsGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions get: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions get: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandConfigFunctionsInventionsGetResponseSchema = z.object({
  remote: RemoteSchema
}).meta({ title: "cli.command.config.functions.inventions.get.Response" });

// src/viewer/command/config/functions/inventions/get.ts
async function configFunctionsInventionsGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/inventions/get" }), z.union([CliErrorSchema, CliCommandConfigFunctionsInventionsGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions inventions get: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsInventionsGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/inventions/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions inventions get: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsInventionsGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/inventions/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions inventions get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsInventionsGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/inventions/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions inventions get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsInventionsGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/inventions/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions inventions get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsInventionsGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/inventions/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions inventions get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsInventionsRemoteGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/inventions/remote/get" }), z.union([CliErrorSchema, RemoteSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions inventions remote get: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsInventionsRemoteGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/inventions/remote/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions inventions remote get: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsInventionsRemoteGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/inventions/remote/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions inventions remote get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsInventionsRemoteGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/inventions/remote/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions inventions remote get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsInventionsRemoteGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/inventions/remote/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions inventions remote get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsInventionsRemoteGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/inventions/remote/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions inventions remote get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsInventionsRemoteSetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/functions/inventions/remote/set" }), z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions inventions remote set: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsInventionsRemoteSetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/functions/inventions/remote/set" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions inventions remote set: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsInventionsRemoteSetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/inventions/remote/set/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions inventions remote set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsInventionsRemoteSetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/inventions/remote/set/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions inventions remote set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsInventionsRemoteSetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/inventions/remote/set/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions inventions remote set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsInventionsRemoteSetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/inventions/remote/set/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions inventions remote set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesFavoritesAddExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/functions/profiles/favorites/add" }), z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles favorites add: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesFavoritesAddExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/functions/profiles/favorites/add" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles favorites add: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesFavoritesAddRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/profiles/favorites/add/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles favorites add request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesFavoritesAddRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/profiles/favorites/add/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles favorites add request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesFavoritesAddResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/profiles/favorites/add/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles favorites add response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesFavoritesAddResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/profiles/favorites/add/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles favorites add response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesFavoritesDelExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/functions/profiles/favorites/del" }), z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles favorites del: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesFavoritesDelExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/functions/profiles/favorites/del" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles favorites del: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesFavoritesDelRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/profiles/favorites/del/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles favorites del request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesFavoritesDelRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/profiles/favorites/del/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles favorites del request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesFavoritesDelResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/profiles/favorites/del/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles favorites del response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesFavoritesDelResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/profiles/favorites/del/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles favorites del response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesFavoritesEditExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/functions/profiles/favorites/edit" }), z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles favorites edit: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesFavoritesEditExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/functions/profiles/favorites/edit" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles favorites edit: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesFavoritesEditRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/profiles/favorites/edit/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles favorites edit request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesFavoritesEditRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/profiles/favorites/edit/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles favorites edit request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesFavoritesEditResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/profiles/favorites/edit/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles favorites edit response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesFavoritesEditResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/profiles/favorites/edit/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles favorites edit response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandConfigFunctionsProfilesFavoritesGetResponseItemSchema = z.union([z.object({
  commit: z.string().nullable().optional(),
  name: z.string(),
  note: z.string(),
  owner: z.string(),
  remote: z.literal("github"),
  repository: z.string()
}).meta({ "variantTitle": "Github" }), z.object({
  commit: z.string().nullable().optional(),
  name: z.string(),
  note: z.string(),
  owner: z.string(),
  remote: z.literal("filesystem"),
  repository: z.string()
}).meta({ "variantTitle": "Filesystem" }), z.object({
  name: z.string(),
  note: z.string(),
  remote: z.literal("mock")
}).meta({ "variantTitle": "Mock" })]).meta({ title: "cli.command.config.functions.profiles.favorites.get.ResponseItem" });

// src/viewer/command/config/functions/profiles/favorites/get.ts
function configFunctionsProfilesFavoritesGetExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/profiles/favorites/get" }), z.union([CliErrorSchema, CliCommandConfigFunctionsProfilesFavoritesGetResponseItemSchema]));
}
function configFunctionsProfilesFavoritesGetExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/profiles/favorites/get" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function configFunctionsProfilesFavoritesGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/profiles/favorites/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles favorites get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesFavoritesGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/profiles/favorites/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles favorites get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesFavoritesGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/profiles/favorites/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles favorites get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesFavoritesGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/profiles/favorites/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles favorites get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandConfigFunctionsProfilesGetResponseSchema = z.object({
  favorites: z.array(CliCommandConfigFunctionsProfilesFavoritesGetResponseItemSchema).nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.config.functions.profiles.get.Response" });

// src/viewer/command/config/functions/profiles/get.ts
async function configFunctionsProfilesGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/profiles/get" }), z.union([CliErrorSchema, CliCommandConfigFunctionsProfilesGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles get: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/profiles/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles get: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/profiles/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/profiles/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/profiles/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/profiles/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsFavoritesAddExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/functions/profiles/pairs/favorites/add" }), z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs favorites add: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsFavoritesAddExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/functions/profiles/pairs/favorites/add" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs favorites add: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsFavoritesAddRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/profiles/pairs/favorites/add/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs favorites add request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsFavoritesAddRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/profiles/pairs/favorites/add/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs favorites add request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsFavoritesAddResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/profiles/pairs/favorites/add/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs favorites add response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsFavoritesAddResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/profiles/pairs/favorites/add/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs favorites add response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsFavoritesDelExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/functions/profiles/pairs/favorites/del" }), z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs favorites del: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsFavoritesDelExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/functions/profiles/pairs/favorites/del" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs favorites del: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsFavoritesDelRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/profiles/pairs/favorites/del/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs favorites del request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsFavoritesDelRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/profiles/pairs/favorites/del/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs favorites del request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsFavoritesDelResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/profiles/pairs/favorites/del/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs favorites del response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsFavoritesDelResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/profiles/pairs/favorites/del/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs favorites del response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsFavoritesEditExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/functions/profiles/pairs/favorites/edit" }), z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs favorites edit: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsFavoritesEditExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/functions/profiles/pairs/favorites/edit" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs favorites edit: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsFavoritesEditRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/profiles/pairs/favorites/edit/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs favorites edit request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsFavoritesEditRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/profiles/pairs/favorites/edit/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs favorites edit request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsFavoritesEditResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/profiles/pairs/favorites/edit/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs favorites edit response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsFavoritesEditResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/profiles/pairs/favorites/edit/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs favorites edit response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandConfigFunctionsProfilesPairsFavoritesGetResponseItemSchema = z.object({
  function: RemotePathCommitOptionalSchema,
  name: z.string(),
  note: z.string(),
  profile: RemotePathCommitOptionalSchema
}).meta({ title: "cli.command.config.functions.profiles.pairs.favorites.get.ResponseItem" });

// src/viewer/command/config/functions/profiles/pairs/favorites/get.ts
function configFunctionsProfilesPairsFavoritesGetExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/profiles/pairs/favorites/get" }), z.union([CliErrorSchema, CliCommandConfigFunctionsProfilesPairsFavoritesGetResponseItemSchema]));
}
function configFunctionsProfilesPairsFavoritesGetExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/profiles/pairs/favorites/get" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function configFunctionsProfilesPairsFavoritesGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/profiles/pairs/favorites/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs favorites get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsFavoritesGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/profiles/pairs/favorites/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs favorites get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsFavoritesGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/profiles/pairs/favorites/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs favorites get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsFavoritesGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/profiles/pairs/favorites/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs favorites get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandConfigFunctionsProfilesPairsGetResponseSchema = z.object({
  favorites: z.array(CliCommandConfigFunctionsProfilesPairsFavoritesGetResponseItemSchema).nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.config.functions.profiles.pairs.get.Response" });

// src/viewer/command/config/functions/profiles/pairs/get.ts
async function configFunctionsProfilesPairsGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/profiles/pairs/get" }), z.union([CliErrorSchema, CliCommandConfigFunctionsProfilesPairsGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs get: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/profiles/pairs/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs get: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/profiles/pairs/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/profiles/pairs/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/functions/profiles/pairs/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configFunctionsProfilesPairsGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/functions/profiles/pairs/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config functions profiles pairs get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandConfigMcpAddressGetResponseSchema = z.object({
  address: z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.config.mcp.address.get.Response" });

// src/viewer/command/config/mcp/address/get.ts
async function configMcpAddressGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/mcp/address/get" }), z.union([CliErrorSchema, CliCommandConfigMcpAddressGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp address get: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpAddressGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/mcp/address/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp address get: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpAddressGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/mcp/address/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp address get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpAddressGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/mcp/address/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp address get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpAddressGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/mcp/address/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp address get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpAddressGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/mcp/address/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp address get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpAddressSetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/mcp/address/set" }), z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp address set: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpAddressSetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/mcp/address/set" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp address set: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpAddressSetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/mcp/address/set/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp address set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpAddressSetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/mcp/address/set/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp address set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpAddressSetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/mcp/address/set/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp address set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpAddressSetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/mcp/address/set/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp address set response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandConfigMcpGetResponseSchema = z.object({
  address: z.string().nullable().meta({ omitempty: true }).optional(),
  port: z.number().int().min(0).max(65535).nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.config.mcp.get.Response" });

// src/viewer/command/config/mcp/get.ts
async function configMcpGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/mcp/get" }), z.union([CliErrorSchema, CliCommandConfigMcpGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp get: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/mcp/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp get: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/mcp/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/mcp/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/mcp/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/mcp/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandConfigMcpPortGetResponseSchema = z.object({
  port: z.number().int().min(0).max(65535)
}).meta({ title: "cli.command.config.mcp.port.get.Response" });

// src/viewer/command/config/mcp/port/get.ts
async function configMcpPortGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/mcp/port/get" }), z.union([CliErrorSchema, CliCommandConfigMcpPortGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp port get: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpPortGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/mcp/port/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp port get: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpPortGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/mcp/port/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp port get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpPortGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/mcp/port/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp port get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpPortGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/mcp/port/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp port get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpPortGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/mcp/port/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp port get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpPortSetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/mcp/port/set" }), z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp port set: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpPortSetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/mcp/port/set" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp port set: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpPortSetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/mcp/port/set/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp port set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpPortSetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/mcp/port/set/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp port set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpPortSetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/mcp/port/set/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp port set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configMcpPortSetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/mcp/port/set/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config mcp port set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsFavoritesAddExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/swarms/favorites/add" }), z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms favorites add: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsFavoritesAddExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/swarms/favorites/add" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms favorites add: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsFavoritesAddRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/swarms/favorites/add/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms favorites add request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsFavoritesAddRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/swarms/favorites/add/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms favorites add request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsFavoritesAddResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/swarms/favorites/add/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms favorites add response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsFavoritesAddResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/swarms/favorites/add/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms favorites add response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsFavoritesDelExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/swarms/favorites/del" }), z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms favorites del: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsFavoritesDelExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/swarms/favorites/del" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms favorites del: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsFavoritesDelRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/swarms/favorites/del/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms favorites del request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsFavoritesDelRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/swarms/favorites/del/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms favorites del request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsFavoritesDelResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/swarms/favorites/del/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms favorites del response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsFavoritesDelResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/swarms/favorites/del/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms favorites del response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsFavoritesEditExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/swarms/favorites/edit" }), z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms favorites edit: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsFavoritesEditExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/swarms/favorites/edit" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms favorites edit: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsFavoritesEditRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/swarms/favorites/edit/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms favorites edit request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsFavoritesEditRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/swarms/favorites/edit/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms favorites edit request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsFavoritesEditResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/swarms/favorites/edit/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms favorites edit response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsFavoritesEditResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/swarms/favorites/edit/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms favorites edit response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandConfigSwarmsFavoritesGetResponseItemSchema = z.union([z.object({
  commit: z.string().nullable().optional(),
  name: z.string(),
  note: z.string(),
  owner: z.string(),
  remote: z.literal("github"),
  repository: z.string()
}).meta({ "variantTitle": "Github" }), z.object({
  commit: z.string().nullable().optional(),
  name: z.string(),
  note: z.string(),
  owner: z.string(),
  remote: z.literal("filesystem"),
  repository: z.string()
}).meta({ "variantTitle": "Filesystem" }), z.object({
  name: z.string(),
  note: z.string(),
  remote: z.literal("mock")
}).meta({ "variantTitle": "Mock" })]).meta({ title: "cli.command.config.swarms.favorites.get.ResponseItem" });

// src/viewer/command/config/swarms/favorites/get.ts
function configSwarmsFavoritesGetExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/swarms/favorites/get" }), z.union([CliErrorSchema, CliCommandConfigSwarmsFavoritesGetResponseItemSchema]));
}
function configSwarmsFavoritesGetExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/swarms/favorites/get" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function configSwarmsFavoritesGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/swarms/favorites/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms favorites get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsFavoritesGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/swarms/favorites/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms favorites get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsFavoritesGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/swarms/favorites/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms favorites get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsFavoritesGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/swarms/favorites/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms favorites get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandConfigSwarmsGetResponseSchema = z.object({
  favorites: z.array(CliCommandConfigSwarmsFavoritesGetResponseItemSchema).nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.config.swarms.get.Response" });

// src/viewer/command/config/swarms/get.ts
async function configSwarmsGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/swarms/get" }), z.union([CliErrorSchema, CliCommandConfigSwarmsGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms get: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/swarms/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms get: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/swarms/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/swarms/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/swarms/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configSwarmsGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/swarms/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config swarms get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandConfigViewerAddressGetResponseSchema = z.object({
  address: z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.config.viewer.address.get.Response" });

// src/viewer/command/config/viewer/address/get.ts
async function configViewerAddressGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/viewer/address/get" }), z.union([CliErrorSchema, CliCommandConfigViewerAddressGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer address get: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerAddressGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/viewer/address/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer address get: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerAddressGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/viewer/address/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer address get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerAddressGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/viewer/address/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer address get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerAddressGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/viewer/address/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer address get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerAddressGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/viewer/address/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer address get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerAddressSetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/viewer/address/set" }), z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer address set: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerAddressSetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/viewer/address/set" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer address set: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerAddressSetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/viewer/address/set/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer address set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerAddressSetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/viewer/address/set/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer address set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerAddressSetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/viewer/address/set/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer address set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerAddressSetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/viewer/address/set/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer address set response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandConfigViewerGetResponseSchema = z.object({
  address: z.string().nullable().meta({ omitempty: true }).optional(),
  port: z.number().int().min(0).max(65535).nullable().meta({ omitempty: true }).optional(),
  secret: z.string().nullable().meta({ omitempty: true }).optional(),
  signature: z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.config.viewer.get.Response" });

// src/viewer/command/config/viewer/get.ts
async function configViewerGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/viewer/get" }), z.union([CliErrorSchema, CliCommandConfigViewerGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer get: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/viewer/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer get: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/viewer/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/viewer/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/viewer/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/viewer/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandConfigViewerPortGetResponseSchema = z.object({
  port: z.number().int().min(0).max(65535)
}).meta({ title: "cli.command.config.viewer.port.get.Response" });

// src/viewer/command/config/viewer/port/get.ts
async function configViewerPortGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/viewer/port/get" }), z.union([CliErrorSchema, CliCommandConfigViewerPortGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer port get: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerPortGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/viewer/port/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer port get: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerPortGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/viewer/port/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer port get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerPortGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/viewer/port/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer port get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerPortGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/viewer/port/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer port get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerPortGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/viewer/port/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer port get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerPortSetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/viewer/port/set" }), z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer port set: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerPortSetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/viewer/port/set" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer port set: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerPortSetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/viewer/port/set/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer port set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerPortSetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/viewer/port/set/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer port set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerPortSetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/viewer/port/set/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer port set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerPortSetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/viewer/port/set/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer port set response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandConfigViewerSecretGetResponseSchema = z.object({
  secret: z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.config.viewer.secret.get.Response" });

// src/viewer/command/config/viewer/secret/get.ts
async function configViewerSecretGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/viewer/secret/get" }), z.union([CliErrorSchema, CliCommandConfigViewerSecretGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer secret get: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerSecretGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/viewer/secret/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer secret get: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerSecretGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/viewer/secret/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer secret get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerSecretGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/viewer/secret/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer secret get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerSecretGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/viewer/secret/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer secret get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerSecretGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/viewer/secret/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer secret get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerSecretSetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/viewer/secret/set" }), z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer secret set: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerSecretSetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/viewer/secret/set" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer secret set: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerSecretSetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/viewer/secret/set/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer secret set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerSecretSetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/viewer/secret/set/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer secret set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerSecretSetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/viewer/secret/set/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer secret set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerSecretSetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/viewer/secret/set/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer secret set response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandConfigViewerSignatureGetResponseSchema = z.object({
  signature: z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.config.viewer.signature.get.Response" });

// src/viewer/command/config/viewer/signature/get.ts
async function configViewerSignatureGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/viewer/signature/get" }), z.union([CliErrorSchema, CliCommandConfigViewerSignatureGetResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer signature get: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerSignatureGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/viewer/signature/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer signature get: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerSignatureGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/viewer/signature/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer signature get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerSignatureGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/viewer/signature/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer signature get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerSignatureGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/viewer/signature/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer signature get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerSignatureGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/viewer/signature/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer signature get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerSignatureSetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/viewer/signature/set" }), z.union([CliErrorSchema, CliCommandOkSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer signature set: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerSignatureSetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, path_type: "config/viewer/signature/set" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer signature set: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerSignatureSetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/viewer/signature/set/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer signature set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerSignatureSetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/viewer/signature/set/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer signature set request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerSignatureSetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "config/viewer/signature/set/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer signature set response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function configViewerSignatureSetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "config/viewer/signature/set/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("config viewer signature set response_schema: cli produced no output before the end marker");
  }
  return first;
}
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
  retry_token: z.string().nullable().meta({ omitempty: true }).optional(),
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
  retry_token: z.string().nullable().meta({ omitempty: true }).optional(),
  tasks: z.array(FunctionsExecutionsResponseStreamingTaskChunkSchema),
  tasks_errors: z.boolean().nullable().meta({ omitempty: true }).optional(),
  usage: AgentCompletionsResponseUsageSchema.nullable().meta({ omitempty: true }).optional()
}).meta({ title: "functions.executions.response.streaming.FunctionExecutionChunk" });

// src/cli/command/functions/executions/create/standard/responseItem.ts
var CliCommandFunctionsExecutionsCreateStandardResponseItemSchema = z.union([FunctionsExecutionsResponseStreamingFunctionExecutionChunkSchema.meta({ "title": "functions.executions.response.streaming.FunctionExecutionChunk", "variantTitle": "Chunk" }), z.string().meta({ "variantTitle": "Id" })]).meta({ title: "cli.command.functions.executions.create.standard.ResponseItem" });

// src/viewer/command/functions/executions/create/standard.ts
function functionsExecutionsCreateStandardExecuteStreaming(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, dangerous_advanced: { ...request.dangerous_advanced ?? {}, stream: true }, path_type: "functions/executions/create/standard" }), z.union([CliErrorSchema, CliCommandFunctionsExecutionsCreateStandardResponseItemSchema]));
}
function functionsExecutionsCreateStandardExecuteStreamingJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, dangerous_advanced: { ...request.dangerous_advanced ?? {}, stream: true }, path_type: "functions/executions/create/standard" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function functionsExecutionsCreateStandardExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, dangerous_advanced: request.dangerous_advanced ? { ...request.dangerous_advanced, stream: void 0 } : request.dangerous_advanced, path_type: "functions/executions/create/standard" }), z.union([CliErrorSchema, z.string()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions executions create standard: cli produced no output before the end marker");
  }
  return first;
}
async function functionsExecutionsCreateStandardExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, dangerous_advanced: request.dangerous_advanced ? { ...request.dangerous_advanced, stream: void 0 } : request.dangerous_advanced, path_type: "functions/executions/create/standard" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions executions create standard: cli produced no output before the end marker");
  }
  return first;
}
async function functionsExecutionsCreateStandardRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/executions/create/standard/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions executions create standard request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsExecutionsCreateStandardRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/executions/create/standard/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions executions create standard request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsExecutionsCreateStandardResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/executions/create/standard/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions executions create standard response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsExecutionsCreateStandardResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/executions/create/standard/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions executions create standard response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandFunctionsExecutionsCreateSwissSystemResponseItemSchema = z.union([FunctionsExecutionsResponseStreamingFunctionExecutionChunkSchema.meta({ "title": "functions.executions.response.streaming.FunctionExecutionChunk", "variantTitle": "Chunk" }), z.string().meta({ "variantTitle": "Id" })]).meta({ title: "cli.command.functions.executions.create.swiss_system.ResponseItem" });

// src/viewer/command/functions/executions/create/swiss_system.ts
function functionsExecutionsCreateSwissSystemExecuteStreaming(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, dangerous_advanced: { ...request.dangerous_advanced ?? {}, stream: true }, path_type: "functions/executions/create/swiss_system" }), z.union([CliErrorSchema, CliCommandFunctionsExecutionsCreateSwissSystemResponseItemSchema]));
}
function functionsExecutionsCreateSwissSystemExecuteStreamingJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, dangerous_advanced: { ...request.dangerous_advanced ?? {}, stream: true }, path_type: "functions/executions/create/swiss_system" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function functionsExecutionsCreateSwissSystemExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, dangerous_advanced: request.dangerous_advanced ? { ...request.dangerous_advanced, stream: void 0 } : request.dangerous_advanced, path_type: "functions/executions/create/swiss_system" }), z.union([CliErrorSchema, z.string()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions executions create swiss_system: cli produced no output before the end marker");
  }
  return first;
}
async function functionsExecutionsCreateSwissSystemExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, dangerous_advanced: request.dangerous_advanced ? { ...request.dangerous_advanced, stream: void 0 } : request.dangerous_advanced, path_type: "functions/executions/create/swiss_system" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions executions create swiss_system: cli produced no output before the end marker");
  }
  return first;
}
async function functionsExecutionsCreateSwissSystemRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/executions/create/swiss_system/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions executions create swiss_system request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsExecutionsCreateSwissSystemRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/executions/create/swiss_system/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions executions create swiss_system request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsExecutionsCreateSwissSystemResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/executions/create/swiss_system/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions executions create swiss_system response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsExecutionsCreateSwissSystemResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/executions/create/swiss_system/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions executions create swiss_system response_schema: cli produced no output before the end marker");
  }
  return first;
}
var FunctionsGetFunctionResponseSchema = RemotePathSchema.and(z.object({})).meta({ title: "functions.GetFunctionResponse" });

// src/viewer/command/functions/get.ts
async function functionsGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/get" }), z.union([CliErrorSchema, FunctionsGetFunctionResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions get: cli produced no output before the end marker");
  }
  return first;
}
async function functionsGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions get: cli produced no output before the end marker");
  }
  return first;
}
async function functionsGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var FunctionsInventionsResponseStreamingAgentCompletionChunkSchema = z.object({
  agent_full_id: z.string().describe("WF-level id: concatenation of the primary agent's id with all\nfallback ids (see `InlineAgentWithFallbacks::full_id`). Same\nfor every slot in the same WF request."),
  agent_id: z.string().describe("Leaf agent id of the slot that produced this chunk. For the\nprimary attempt this is the primary agent's id; on fallback it\nis the fallback agent's id. Same on every chunk of a slot."),
  agent_instance_hierarchy: z.string().describe("Full agent instance hierarchy for this completion's slot \u2014\n`{ctx lineage}/{agent_full_id}-{response_id}`, or the fixed\ncontinuation value on resume. Same on every chunk of a slot."),
  agent_remote: RemotePathSchema.nullable().describe("`RemotePath` the WF was fetched from. `None` when the WF was\nsupplied inline. Same for every slot in the same WF request.").meta({ omitempty: true }).optional(),
  continuation: z.string().nullable().describe("Continuation state for multi-turn conversations (only present in the final chunk).").meta({ omitempty: true }).optional(),
  created: z.number().int().min(0).max(18446744073709552e3),
  error: ErrorResponseErrorSchema.nullable().describe("Error details if this completion failed.").meta({ omitempty: true }).optional(),
  id: z.string(),
  index: z.number().int().min(0).max(18446744073709552e3),
  messages: z.array(AgentCompletionsResponseStreamingMessageChunkSchema),
  messages_queued: z.boolean().nullable().describe("`true` when the MCP proxy holds queued messages that were not\ndelivered to the agent via a tool response on this turn. Only\nset when `continuation` is also set \u2014 the caller acts on it by\nissuing the continuation. Absent when nothing is queued, when\nthere is no continuation to act on, or when the peek failed\n(the failure is surfaced via `error`).").meta({ omitempty: true }).optional(),
  object: AgentCompletionsResponseStreamingObjectSchema.describe('The object type (always "agent.completion.chunk").'),
  upstream: AgentUpstreamSchema.describe("Upstream provider"),
  usage: AgentCompletionsResponseUsageSchema.nullable().describe("Token usage (only present in the final chunk).").meta({ omitempty: true }).optional()
}).describe("A chunk of a streaming agent completion response.\n\nMultiple chunks are received via Server-Sent Events and can be\naccumulated into a complete [`AgentCompletion`](response::unary::AgentCompletion)\nusing the [`push`](Self::push) method.").meta({ title: "functions.inventions.response.streaming.AgentCompletionChunk" });

// src/functions/inventions/recursive/response/streaming/functionInventionChunk.ts
var FunctionsInventionsRecursiveResponseStreamingFunctionInventionChunkSchema = z.object({
  completions: z.array(FunctionsInventionsResponseStreamingAgentCompletionChunkSchema),
  created: z.number().int().min(0).max(18446744073709552e3),
  error: ErrorResponseErrorSchema.nullable().meta({ omitempty: true }).optional(),
  function: FunctionsFullRemoteFunctionSchema.nullable().meta({ omitempty: true }).optional(),
  id: z.string(),
  index: z.number().int().min(0).max(18446744073709552e3),
  object: FunctionsInventionsResponseStreamingObjectSchema,
  path: RemotePathSchema.nullable().meta({ omitempty: true }).optional(),
  state: FunctionsInventionsStateStateSchema.nullable().meta({ omitempty: true }).optional(),
  usage: AgentCompletionsResponseUsageSchema.nullable().meta({ omitempty: true }).optional()
}).meta({ title: "functions.inventions.recursive.response.streaming.FunctionInventionChunk" });

// src/functions/inventions/recursive/response/streaming/functionInventionRecursiveChunk.ts
var FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunkSchema = z.object({
  created: z.number().int().min(0).max(18446744073709552e3),
  id: z.string(),
  inventions: z.array(FunctionsInventionsRecursiveResponseStreamingFunctionInventionChunkSchema),
  inventions_errors: z.boolean().nullable().meta({ omitempty: true }).optional(),
  object: FunctionsInventionsRecursiveResponseStreamingObjectSchema,
  usage: AgentCompletionsResponseUsageSchema.nullable().meta({ omitempty: true }).optional()
}).meta({ title: "functions.inventions.recursive.response.streaming.FunctionInventionRecursiveChunk" });

// src/cli/command/functions/inventions/recursive/create/alpha_scalar/responseItem.ts
var CliCommandFunctionsInventionsRecursiveCreateAlphaScalarResponseItemSchema = z.union([FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunkSchema.meta({ "title": "functions.inventions.recursive.response.streaming.FunctionInventionRecursiveChunk", "variantTitle": "Chunk" }), z.string().meta({ "variantTitle": "Id" })]).meta({ title: "cli.command.functions.inventions.recursive.create.alpha_scalar.ResponseItem" });

// src/viewer/command/functions/inventions/recursive/create/alpha_scalar.ts
function functionsInventionsRecursiveCreateAlphaScalarExecuteStreaming(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, dangerous_advanced: { ...request.dangerous_advanced ?? {}, stream: true }, path_type: "functions/inventions/recursive/create/alpha_scalar" }), z.union([CliErrorSchema, CliCommandFunctionsInventionsRecursiveCreateAlphaScalarResponseItemSchema]));
}
function functionsInventionsRecursiveCreateAlphaScalarExecuteStreamingJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, dangerous_advanced: { ...request.dangerous_advanced ?? {}, stream: true }, path_type: "functions/inventions/recursive/create/alpha_scalar" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function functionsInventionsRecursiveCreateAlphaScalarExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, dangerous_advanced: request.dangerous_advanced ? { ...request.dangerous_advanced, stream: void 0 } : request.dangerous_advanced, path_type: "functions/inventions/recursive/create/alpha_scalar" }), z.union([CliErrorSchema, z.string()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions inventions recursive create alpha_scalar: cli produced no output before the end marker");
  }
  return first;
}
async function functionsInventionsRecursiveCreateAlphaScalarExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, dangerous_advanced: request.dangerous_advanced ? { ...request.dangerous_advanced, stream: void 0 } : request.dangerous_advanced, path_type: "functions/inventions/recursive/create/alpha_scalar" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions inventions recursive create alpha_scalar: cli produced no output before the end marker");
  }
  return first;
}
async function functionsInventionsRecursiveCreateAlphaScalarRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/inventions/recursive/create/alpha_scalar/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions inventions recursive create alpha_scalar request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsInventionsRecursiveCreateAlphaScalarRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/inventions/recursive/create/alpha_scalar/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions inventions recursive create alpha_scalar request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsInventionsRecursiveCreateAlphaScalarResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/inventions/recursive/create/alpha_scalar/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions inventions recursive create alpha_scalar response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsInventionsRecursiveCreateAlphaScalarResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/inventions/recursive/create/alpha_scalar/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions inventions recursive create alpha_scalar response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandFunctionsInventionsRecursiveCreateAlphaVectorResponseItemSchema = z.union([FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunkSchema.meta({ "title": "functions.inventions.recursive.response.streaming.FunctionInventionRecursiveChunk", "variantTitle": "Chunk" }), z.string().meta({ "variantTitle": "Id" })]).meta({ title: "cli.command.functions.inventions.recursive.create.alpha_vector.ResponseItem" });

// src/viewer/command/functions/inventions/recursive/create/alpha_vector.ts
function functionsInventionsRecursiveCreateAlphaVectorExecuteStreaming(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, dangerous_advanced: { ...request.dangerous_advanced ?? {}, stream: true }, path_type: "functions/inventions/recursive/create/alpha_vector" }), z.union([CliErrorSchema, CliCommandFunctionsInventionsRecursiveCreateAlphaVectorResponseItemSchema]));
}
function functionsInventionsRecursiveCreateAlphaVectorExecuteStreamingJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, dangerous_advanced: { ...request.dangerous_advanced ?? {}, stream: true }, path_type: "functions/inventions/recursive/create/alpha_vector" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function functionsInventionsRecursiveCreateAlphaVectorExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, dangerous_advanced: request.dangerous_advanced ? { ...request.dangerous_advanced, stream: void 0 } : request.dangerous_advanced, path_type: "functions/inventions/recursive/create/alpha_vector" }), z.union([CliErrorSchema, z.string()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions inventions recursive create alpha_vector: cli produced no output before the end marker");
  }
  return first;
}
async function functionsInventionsRecursiveCreateAlphaVectorExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, dangerous_advanced: request.dangerous_advanced ? { ...request.dangerous_advanced, stream: void 0 } : request.dangerous_advanced, path_type: "functions/inventions/recursive/create/alpha_vector" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions inventions recursive create alpha_vector: cli produced no output before the end marker");
  }
  return first;
}
async function functionsInventionsRecursiveCreateAlphaVectorRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/inventions/recursive/create/alpha_vector/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions inventions recursive create alpha_vector request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsInventionsRecursiveCreateAlphaVectorRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/inventions/recursive/create/alpha_vector/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions inventions recursive create alpha_vector request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsInventionsRecursiveCreateAlphaVectorResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/inventions/recursive/create/alpha_vector/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions inventions recursive create alpha_vector response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsInventionsRecursiveCreateAlphaVectorResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/inventions/recursive/create/alpha_vector/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions inventions recursive create alpha_vector response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandFunctionsInventionsRecursiveCreateRemoteResponseItemSchema = z.union([FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunkSchema.meta({ "title": "functions.inventions.recursive.response.streaming.FunctionInventionRecursiveChunk", "variantTitle": "Chunk" }), z.string().meta({ "variantTitle": "Id" })]).meta({ title: "cli.command.functions.inventions.recursive.create.remote.ResponseItem" });

// src/viewer/command/functions/inventions/recursive/create/remote.ts
function functionsInventionsRecursiveCreateRemoteExecuteStreaming(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, dangerous_advanced: { ...request.dangerous_advanced ?? {}, stream: true }, path_type: "functions/inventions/recursive/create/remote" }), z.union([CliErrorSchema, CliCommandFunctionsInventionsRecursiveCreateRemoteResponseItemSchema]));
}
function functionsInventionsRecursiveCreateRemoteExecuteStreamingJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, dangerous_advanced: { ...request.dangerous_advanced ?? {}, stream: true }, path_type: "functions/inventions/recursive/create/remote" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function functionsInventionsRecursiveCreateRemoteExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, dangerous_advanced: request.dangerous_advanced ? { ...request.dangerous_advanced, stream: void 0 } : request.dangerous_advanced, path_type: "functions/inventions/recursive/create/remote" }), z.union([CliErrorSchema, z.string()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions inventions recursive create remote: cli produced no output before the end marker");
  }
  return first;
}
async function functionsInventionsRecursiveCreateRemoteExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, dangerous_advanced: request.dangerous_advanced ? { ...request.dangerous_advanced, stream: void 0 } : request.dangerous_advanced, path_type: "functions/inventions/recursive/create/remote" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions inventions recursive create remote: cli produced no output before the end marker");
  }
  return first;
}
async function functionsInventionsRecursiveCreateRemoteRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/inventions/recursive/create/remote/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions inventions recursive create remote request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsInventionsRecursiveCreateRemoteRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/inventions/recursive/create/remote/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions inventions recursive create remote request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsInventionsRecursiveCreateRemoteResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/inventions/recursive/create/remote/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions inventions recursive create remote response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsInventionsRecursiveCreateRemoteResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/inventions/recursive/create/remote/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions inventions recursive create remote response_schema: cli produced no output before the end marker");
  }
  return first;
}
var FunctionsInventionsStateGetFunctionInventionStateResponseSchema = RemotePathSchema.and(z.object({})).describe("Response from retrieving a function invention state.").meta({ title: "functions.inventions.state.GetFunctionInventionStateResponse" });

// src/viewer/command/functions/inventions/state/get.ts
async function functionsInventionsStateGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/inventions/state/get" }), z.union([CliErrorSchema, FunctionsInventionsStateGetFunctionInventionStateResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions inventions state get: cli produced no output before the end marker");
  }
  return first;
}
async function functionsInventionsStateGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/inventions/state/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions inventions state get: cli produced no output before the end marker");
  }
  return first;
}
async function functionsInventionsStateGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/inventions/state/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions inventions state get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsInventionsStateGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/inventions/state/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions inventions state get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsInventionsStateGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/inventions/state/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions inventions state get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsInventionsStateGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/inventions/state/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions inventions state get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandFunctionsListResponseFavoriteSchema = z.union([z.object({
  commit: z.string().nullable().optional(),
  name: z.string(),
  note: z.string(),
  owner: z.string(),
  remote: z.literal("github"),
  repository: z.string()
}).meta({ "variantTitle": "Github" }), z.object({
  commit: z.string().nullable().optional(),
  name: z.string(),
  note: z.string(),
  owner: z.string(),
  remote: z.literal("filesystem"),
  repository: z.string()
}).meta({ "variantTitle": "Filesystem" }), z.object({
  name: z.string(),
  note: z.string(),
  remote: z.literal("mock")
}).meta({ "variantTitle": "Mock" })]).meta({ title: "cli.command.functions.list.ResponseFavorite" });

// src/cli/command/functions/list/responseItem.ts
var CliCommandFunctionsListResponseItemSchema = z.union([CliCommandFunctionsListResponseFavoriteSchema.meta({ "title": "cli.command.functions.list.ResponseFavorite", "variantTitle": "Favorite" }), RemotePathSchema.meta({ "title": "RemotePath", "variantTitle": "Item" })]).meta({ title: "cli.command.functions.list.ResponseItem" });

// src/viewer/command/functions/list.ts
function functionsListExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/list" }), z.union([CliErrorSchema, CliCommandFunctionsListResponseItemSchema]));
}
function functionsListExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/list" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function functionsListRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/list/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsListRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/list/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsListResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/list/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions list response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsListResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/list/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions list response_schema: cli produced no output before the end marker");
  }
  return first;
}
var FunctionsProfilesGetProfileResponseSchema = RemotePathSchema.and(z.object({})).meta({ title: "functions.profiles.GetProfileResponse" });

// src/viewer/command/functions/profiles/get.ts
async function functionsProfilesGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/profiles/get" }), z.union([CliErrorSchema, FunctionsProfilesGetProfileResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles get: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/profiles/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles get: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/profiles/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/profiles/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/profiles/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/profiles/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandFunctionsProfilesListResponseFavoriteSchema = z.union([z.object({
  commit: z.string().nullable().optional(),
  name: z.string(),
  note: z.string(),
  owner: z.string(),
  remote: z.literal("github"),
  repository: z.string()
}).meta({ "variantTitle": "Github" }), z.object({
  commit: z.string().nullable().optional(),
  name: z.string(),
  note: z.string(),
  owner: z.string(),
  remote: z.literal("filesystem"),
  repository: z.string()
}).meta({ "variantTitle": "Filesystem" }), z.object({
  name: z.string(),
  note: z.string(),
  remote: z.literal("mock")
}).meta({ "variantTitle": "Mock" })]).meta({ title: "cli.command.functions.profiles.list.ResponseFavorite" });

// src/cli/command/functions/profiles/list/responseItem.ts
var CliCommandFunctionsProfilesListResponseItemSchema = z.union([CliCommandFunctionsProfilesListResponseFavoriteSchema.meta({ "title": "cli.command.functions.profiles.list.ResponseFavorite", "variantTitle": "Favorite" }), RemotePathSchema.meta({ "title": "RemotePath", "variantTitle": "Item" })]).meta({ title: "cli.command.functions.profiles.list.ResponseItem" });

// src/viewer/command/functions/profiles/list.ts
function functionsProfilesListExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/profiles/list" }), z.union([CliErrorSchema, CliCommandFunctionsProfilesListResponseItemSchema]));
}
function functionsProfilesListExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/profiles/list" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function functionsProfilesListRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/profiles/list/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesListRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/profiles/list/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesListResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/profiles/list/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles list response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesListResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/profiles/list/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
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
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/profiles/publish" }), z.union([CliErrorSchema, CliCommandFunctionsProfilesPublishResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles publish: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesPublishExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/profiles/publish" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles publish: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesPublishRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/profiles/publish/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles publish request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesPublishRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/profiles/publish/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles publish request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesPublishResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/profiles/publish/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions profiles publish response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsProfilesPublishResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/profiles/publish/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
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
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/publish" }), z.union([CliErrorSchema, CliCommandFunctionsPublishResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions publish: cli produced no output before the end marker");
  }
  return first;
}
async function functionsPublishExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/publish" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions publish: cli produced no output before the end marker");
  }
  return first;
}
async function functionsPublishRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/publish/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions publish request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsPublishRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/publish/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions publish request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsPublishResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "functions/publish/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions publish response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function functionsPublishResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "functions/publish/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("functions publish response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/get" }), z.union([CliErrorSchema, AgentCompletionsRequestAgentCompletionCreateParamsLogSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesAudioGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/messages/audio/get" }), z.union([CliErrorSchema, AgentCompletionsMessageInputAudioSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages audio get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesAudioGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/messages/audio/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages audio get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesAudioGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/messages/audio/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages audio get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesAudioGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/messages/audio/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages audio get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesAudioGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/messages/audio/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages audio get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesAudioGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/messages/audio/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages audio get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesFileGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/messages/file/get" }), z.union([CliErrorSchema, AgentCompletionsMessageFileSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages file get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesFileGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/messages/file/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages file get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesFileGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/messages/file/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages file get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesFileGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/messages/file/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages file get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesFileGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/messages/file/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages file get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesFileGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/messages/file/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages file get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesImageGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/messages/image/get" }), z.union([CliErrorSchema, AgentCompletionsMessageImageUrlSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages image get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesImageGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/messages/image/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages image get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesImageGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/messages/image/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages image get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesImageGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/messages/image/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages image get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesImageGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/messages/image/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages image get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesImageGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/messages/image/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages image get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesTextGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/messages/text/get" }), z.union([CliErrorSchema, z.string()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages text get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesTextGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/messages/text/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages text get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesTextGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/messages/text/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages text get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesTextGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/messages/text/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages text get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesTextGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/messages/text/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages text get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesTextGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/messages/text/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages text get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesVideoGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/messages/video/get" }), z.union([CliErrorSchema, AgentCompletionsMessageVideoUrlSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages video get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesVideoGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/messages/video/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages video get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesVideoGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/messages/video/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages video get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesVideoGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/messages/video/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages video get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesVideoGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/messages/video/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages video get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestMessagesVideoGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/messages/video/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request messages video get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsAudioGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/notifications/audio/get" }), z.union([CliErrorSchema, AgentCompletionsMessageInputAudioSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications audio get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsAudioGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/notifications/audio/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications audio get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsAudioGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/notifications/audio/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications audio get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsAudioGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/notifications/audio/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications audio get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsAudioGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/notifications/audio/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications audio get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsAudioGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/notifications/audio/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications audio get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsFileGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/notifications/file/get" }), z.union([CliErrorSchema, AgentCompletionsMessageFileSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications file get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsFileGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/notifications/file/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications file get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsFileGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/notifications/file/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications file get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsFileGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/notifications/file/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications file get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsFileGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/notifications/file/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications file get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsFileGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/notifications/file/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications file get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsImageGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/notifications/image/get" }), z.union([CliErrorSchema, AgentCompletionsMessageImageUrlSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications image get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsImageGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/notifications/image/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications image get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsImageGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/notifications/image/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications image get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsImageGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/notifications/image/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications image get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsImageGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/notifications/image/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications image get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsImageGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/notifications/image/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications image get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsTextGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/notifications/text/get" }), z.union([CliErrorSchema, z.string()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications text get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsTextGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/notifications/text/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications text get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsTextGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/notifications/text/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications text get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsTextGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/notifications/text/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications text get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsTextGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/notifications/text/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications text get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsTextGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/notifications/text/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications text get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsVideoGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/notifications/video/get" }), z.union([CliErrorSchema, AgentCompletionsMessageVideoUrlSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications video get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsVideoGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/notifications/video/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications video get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsVideoGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/notifications/video/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications video get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsVideoGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/notifications/video/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications video get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsVideoGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/notifications/video/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications video get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestNotificationsVideoGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/notifications/video/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request notifications video get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestSubscribeExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/subscribe" }), z.union([CliErrorSchema, AgentCompletionsRequestAgentCompletionCreateParamsLogSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestSubscribeExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/subscribe" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestSubscribeRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/subscribe/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestSubscribeRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/subscribe/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestSubscribeResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/request/subscribe/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsRequestSubscribeResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/request/subscribe/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions request subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandLogsAgentsCompletionsResponseClearResponseSchema = z.object({
  count: z.number().int().min(0).max(18446744073709552e3)
}).meta({ title: "cli.command.logs.agents.completions.response.clear.Response" });

// src/viewer/command/logs/agents/completions/response/clear.ts
async function logsAgentsCompletionsResponseClearExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/clear" }), z.union([CliErrorSchema, CliCommandLogsAgentsCompletionsResponseClearResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseClearExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/clear" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseClearRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/clear/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseClearRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/clear/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseClearResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/clear/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseClearResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/clear/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandLogsAgentsCompletionsResponseContinuationsClearResponseSchema = z.object({
  count: z.number().int().min(0).max(18446744073709552e3)
}).meta({ title: "cli.command.logs.agents.completions.response.continuations.clear.Response" });

// src/viewer/command/logs/agents/completions/response/continuations/clear.ts
async function logsAgentsCompletionsResponseContinuationsClearExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/continuations/clear" }), z.union([CliErrorSchema, CliCommandLogsAgentsCompletionsResponseContinuationsClearResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response continuations clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseContinuationsClearExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/continuations/clear" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response continuations clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseContinuationsClearRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/continuations/clear/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response continuations clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseContinuationsClearRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/continuations/clear/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response continuations clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseContinuationsClearResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/continuations/clear/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response continuations clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseContinuationsClearResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/continuations/clear/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response continuations clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseContinuationsGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/continuations/get" }), z.union([CliErrorSchema, z.string()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response continuations get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseContinuationsGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/continuations/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response continuations get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseContinuationsGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/continuations/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response continuations get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseContinuationsGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/continuations/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response continuations get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseContinuationsGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/continuations/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response continuations get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseContinuationsGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/continuations/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response continuations get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseContinuationsSubscribeExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/continuations/subscribe" }), z.union([CliErrorSchema, z.string()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response continuations subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseContinuationsSubscribeExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/continuations/subscribe" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response continuations subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseContinuationsSubscribeRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/continuations/subscribe/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response continuations subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseContinuationsSubscribeRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/continuations/subscribe/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response continuations subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseContinuationsSubscribeResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/continuations/subscribe/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response continuations subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseContinuationsSubscribeResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/continuations/subscribe/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response continuations subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/get" }), z.union([CliErrorSchema, AgentCompletionsResponseStreamingAgentCompletionChunkLogSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandLogsAgentsCompletionsResponseListResponseItemSchema = z.object({
  created: z.number().int().min(0).max(18446744073709552e3),
  id: z.string()
}).meta({ title: "cli.command.logs.agents.completions.response.list.ResponseItem" });

// src/viewer/command/logs/agents/completions/response/list.ts
function logsAgentsCompletionsResponseListExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/list" }), z.union([CliErrorSchema, CliCommandLogsAgentsCompletionsResponseListResponseItemSchema]));
}
function logsAgentsCompletionsResponseListExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/list" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function logsAgentsCompletionsResponseListRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/list/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseListRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/list/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseListResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/list/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response list response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseListResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/list/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response list response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandLogsAgentsCompletionsResponseMessagesAssistantAudioClearResponseSchema = z.object({
  count: z.number().int().min(0).max(18446744073709552e3)
}).meta({ title: "cli.command.logs.agents.completions.response.messages.assistant.audio.clear.Response" });

// src/viewer/command/logs/agents/completions/response/messages/assistant/audio/clear.ts
async function logsAgentsCompletionsResponseMessagesAssistantAudioClearExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/audio/clear" }), z.union([CliErrorSchema, CliCommandLogsAgentsCompletionsResponseMessagesAssistantAudioClearResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant audio clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantAudioClearExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/audio/clear" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant audio clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantAudioClearRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/audio/clear/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant audio clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantAudioClearRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/audio/clear/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant audio clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantAudioClearResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/audio/clear/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant audio clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantAudioClearResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/audio/clear/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant audio clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantAudioGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/audio/get" }), z.union([CliErrorSchema, AgentCompletionsMessageInputAudioSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant audio get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantAudioGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/audio/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant audio get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantAudioGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/audio/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant audio get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantAudioGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/audio/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant audio get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantAudioGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/audio/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant audio get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantAudioGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/audio/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant audio get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantAudioSubscribeExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/audio/subscribe" }), z.union([CliErrorSchema, AgentCompletionsMessageInputAudioSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant audio subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantAudioSubscribeExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/audio/subscribe" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant audio subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantAudioSubscribeRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/audio/subscribe/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant audio subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantAudioSubscribeRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/audio/subscribe/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant audio subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantAudioSubscribeResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/audio/subscribe/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant audio subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantAudioSubscribeResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/audio/subscribe/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant audio subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandLogsAgentsCompletionsResponseMessagesAssistantClearResponseSchema = z.object({
  count: z.number().int().min(0).max(18446744073709552e3)
}).meta({ title: "cli.command.logs.agents.completions.response.messages.assistant.clear.Response" });

// src/viewer/command/logs/agents/completions/response/messages/assistant/clear.ts
async function logsAgentsCompletionsResponseMessagesAssistantClearExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/clear" }), z.union([CliErrorSchema, CliCommandLogsAgentsCompletionsResponseMessagesAssistantClearResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantClearExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/clear" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantClearRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/clear/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantClearRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/clear/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantClearResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/clear/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantClearResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/clear/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandLogsAgentsCompletionsResponseMessagesAssistantFileClearResponseSchema = z.object({
  count: z.number().int().min(0).max(18446744073709552e3)
}).meta({ title: "cli.command.logs.agents.completions.response.messages.assistant.file.clear.Response" });

// src/viewer/command/logs/agents/completions/response/messages/assistant/file/clear.ts
async function logsAgentsCompletionsResponseMessagesAssistantFileClearExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/file/clear" }), z.union([CliErrorSchema, CliCommandLogsAgentsCompletionsResponseMessagesAssistantFileClearResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant file clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantFileClearExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/file/clear" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant file clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantFileClearRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/file/clear/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant file clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantFileClearRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/file/clear/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant file clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantFileClearResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/file/clear/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant file clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantFileClearResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/file/clear/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant file clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantFileGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/file/get" }), z.union([CliErrorSchema, AgentCompletionsMessageFileSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant file get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantFileGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/file/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant file get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantFileGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/file/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant file get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantFileGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/file/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant file get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantFileGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/file/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant file get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantFileGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/file/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant file get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantFileSubscribeExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/file/subscribe" }), z.union([CliErrorSchema, AgentCompletionsMessageFileSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant file subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantFileSubscribeExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/file/subscribe" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant file subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantFileSubscribeRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/file/subscribe/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant file subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantFileSubscribeRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/file/subscribe/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant file subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantFileSubscribeResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/file/subscribe/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant file subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantFileSubscribeResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/file/subscribe/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant file subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/get" }), z.union([CliErrorSchema, AgentCompletionsResponseStreamingAssistantResponseChunkLogSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandLogsAgentsCompletionsResponseMessagesAssistantImageClearResponseSchema = z.object({
  count: z.number().int().min(0).max(18446744073709552e3)
}).meta({ title: "cli.command.logs.agents.completions.response.messages.assistant.image.clear.Response" });

// src/viewer/command/logs/agents/completions/response/messages/assistant/image/clear.ts
async function logsAgentsCompletionsResponseMessagesAssistantImageClearExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/image/clear" }), z.union([CliErrorSchema, CliCommandLogsAgentsCompletionsResponseMessagesAssistantImageClearResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant image clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantImageClearExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/image/clear" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant image clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantImageClearRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/image/clear/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant image clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantImageClearRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/image/clear/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant image clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantImageClearResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/image/clear/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant image clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantImageClearResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/image/clear/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant image clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantImageGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/image/get" }), z.union([CliErrorSchema, AgentCompletionsMessageImageUrlSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant image get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantImageGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/image/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant image get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantImageGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/image/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant image get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantImageGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/image/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant image get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantImageGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/image/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant image get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantImageGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/image/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant image get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantImageSubscribeExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/image/subscribe" }), z.union([CliErrorSchema, AgentCompletionsMessageImageUrlSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant image subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantImageSubscribeExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/image/subscribe" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant image subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantImageSubscribeRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/image/subscribe/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant image subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantImageSubscribeRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/image/subscribe/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant image subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantImageSubscribeResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/image/subscribe/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant image subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantImageSubscribeResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/image/subscribe/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant image subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandLogsAgentsCompletionsResponseMessagesAssistantLogprobsClearResponseSchema = z.object({
  count: z.number().int().min(0).max(18446744073709552e3)
}).meta({ title: "cli.command.logs.agents.completions.response.messages.assistant.logprobs.clear.Response" });

// src/viewer/command/logs/agents/completions/response/messages/assistant/logprobs/clear.ts
async function logsAgentsCompletionsResponseMessagesAssistantLogprobsClearExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/logprobs/clear" }), z.union([CliErrorSchema, CliCommandLogsAgentsCompletionsResponseMessagesAssistantLogprobsClearResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant logprobs clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantLogprobsClearExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/logprobs/clear" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant logprobs clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantLogprobsClearRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/logprobs/clear/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant logprobs clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantLogprobsClearRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/logprobs/clear/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant logprobs clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantLogprobsClearResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/logprobs/clear/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant logprobs clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantLogprobsClearResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/logprobs/clear/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant logprobs clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantLogprobsGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/logprobs/get" }), z.union([CliErrorSchema, AgentCompletionsResponseLogprobsSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant logprobs get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantLogprobsGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/logprobs/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant logprobs get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantLogprobsGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/logprobs/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant logprobs get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantLogprobsGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/logprobs/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant logprobs get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantLogprobsGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/logprobs/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant logprobs get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantLogprobsGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/logprobs/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant logprobs get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantLogprobsSubscribeExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/logprobs/subscribe" }), z.union([CliErrorSchema, AgentCompletionsResponseLogprobsSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant logprobs subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantLogprobsSubscribeExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/logprobs/subscribe" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant logprobs subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantLogprobsSubscribeRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/logprobs/subscribe/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant logprobs subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantLogprobsSubscribeRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/logprobs/subscribe/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant logprobs subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantLogprobsSubscribeResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/logprobs/subscribe/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant logprobs subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantLogprobsSubscribeResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/logprobs/subscribe/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant logprobs subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandLogsAgentsCompletionsResponseMessagesAssistantReasoningClearResponseSchema = z.object({
  count: z.number().int().min(0).max(18446744073709552e3)
}).meta({ title: "cli.command.logs.agents.completions.response.messages.assistant.reasoning.clear.Response" });

// src/viewer/command/logs/agents/completions/response/messages/assistant/reasoning/clear.ts
async function logsAgentsCompletionsResponseMessagesAssistantReasoningClearExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/reasoning/clear" }), z.union([CliErrorSchema, CliCommandLogsAgentsCompletionsResponseMessagesAssistantReasoningClearResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant reasoning clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantReasoningClearExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/reasoning/clear" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant reasoning clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantReasoningClearRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/reasoning/clear/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant reasoning clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantReasoningClearRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/reasoning/clear/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant reasoning clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantReasoningClearResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/reasoning/clear/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant reasoning clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantReasoningClearResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/reasoning/clear/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant reasoning clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantReasoningGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/reasoning/get" }), z.union([CliErrorSchema, z.string()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant reasoning get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantReasoningGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/reasoning/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant reasoning get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantReasoningGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/reasoning/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant reasoning get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantReasoningGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/reasoning/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant reasoning get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantReasoningGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/reasoning/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant reasoning get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantReasoningGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/reasoning/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant reasoning get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantReasoningSubscribeExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/reasoning/subscribe" }), z.union([CliErrorSchema, z.string()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant reasoning subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantReasoningSubscribeExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/reasoning/subscribe" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant reasoning subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantReasoningSubscribeRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/reasoning/subscribe/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant reasoning subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantReasoningSubscribeRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/reasoning/subscribe/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant reasoning subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantReasoningSubscribeResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/reasoning/subscribe/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant reasoning subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantReasoningSubscribeResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/reasoning/subscribe/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant reasoning subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandLogsAgentsCompletionsResponseMessagesAssistantRefusalClearResponseSchema = z.object({
  count: z.number().int().min(0).max(18446744073709552e3)
}).meta({ title: "cli.command.logs.agents.completions.response.messages.assistant.refusal.clear.Response" });

// src/viewer/command/logs/agents/completions/response/messages/assistant/refusal/clear.ts
async function logsAgentsCompletionsResponseMessagesAssistantRefusalClearExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/refusal/clear" }), z.union([CliErrorSchema, CliCommandLogsAgentsCompletionsResponseMessagesAssistantRefusalClearResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant refusal clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantRefusalClearExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/refusal/clear" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant refusal clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantRefusalClearRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/refusal/clear/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant refusal clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantRefusalClearRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/refusal/clear/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant refusal clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantRefusalClearResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/refusal/clear/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant refusal clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantRefusalClearResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/refusal/clear/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant refusal clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantRefusalGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/refusal/get" }), z.union([CliErrorSchema, z.string()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant refusal get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantRefusalGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/refusal/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant refusal get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantRefusalGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/refusal/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant refusal get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantRefusalGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/refusal/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant refusal get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantRefusalGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/refusal/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant refusal get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantRefusalGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/refusal/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant refusal get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantRefusalSubscribeExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/refusal/subscribe" }), z.union([CliErrorSchema, z.string()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant refusal subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantRefusalSubscribeExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/refusal/subscribe" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant refusal subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantRefusalSubscribeRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/refusal/subscribe/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant refusal subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantRefusalSubscribeRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/refusal/subscribe/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant refusal subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantRefusalSubscribeResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/refusal/subscribe/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant refusal subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantRefusalSubscribeResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/refusal/subscribe/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant refusal subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantSubscribeExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/subscribe" }), z.union([CliErrorSchema, AgentCompletionsResponseStreamingAssistantResponseChunkLogSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantSubscribeExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/subscribe" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantSubscribeRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/subscribe/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantSubscribeRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/subscribe/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantSubscribeResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/subscribe/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantSubscribeResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/subscribe/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantTextGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/text/get" }), z.union([CliErrorSchema, z.string()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant text get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantTextGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/text/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant text get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantTextGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/text/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant text get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantTextGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/text/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant text get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantTextGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/text/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant text get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantTextGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/text/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant text get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandLogsAgentsCompletionsResponseMessagesAssistantToolCallsClearResponseSchema = z.object({
  count: z.number().int().min(0).max(18446744073709552e3)
}).meta({ title: "cli.command.logs.agents.completions.response.messages.assistant.tool_calls.clear.Response" });

// src/viewer/command/logs/agents/completions/response/messages/assistant/tool_calls/clear.ts
async function logsAgentsCompletionsResponseMessagesAssistantToolCallsClearExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/tool_calls/clear" }), z.union([CliErrorSchema, CliCommandLogsAgentsCompletionsResponseMessagesAssistantToolCallsClearResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant tool_calls clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantToolCallsClearExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/tool_calls/clear" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant tool_calls clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantToolCallsClearRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/tool_calls/clear/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant tool_calls clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantToolCallsClearRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/tool_calls/clear/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant tool_calls clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantToolCallsClearResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/tool_calls/clear/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant tool_calls clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantToolCallsClearResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/tool_calls/clear/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant tool_calls clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantToolCallsGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/tool_calls/get" }), z.union([CliErrorSchema, AgentCompletionsMessageAssistantToolCallDeltaSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant tool_calls get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantToolCallsGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/tool_calls/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant tool_calls get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantToolCallsGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/tool_calls/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant tool_calls get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantToolCallsGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/tool_calls/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant tool_calls get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantToolCallsGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/tool_calls/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant tool_calls get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantToolCallsGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/tool_calls/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant tool_calls get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantToolCallsSubscribeExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/tool_calls/subscribe" }), z.union([CliErrorSchema, AgentCompletionsMessageAssistantToolCallDeltaSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant tool_calls subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantToolCallsSubscribeExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/tool_calls/subscribe" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant tool_calls subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantToolCallsSubscribeRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/tool_calls/subscribe/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant tool_calls subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantToolCallsSubscribeRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/tool_calls/subscribe/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant tool_calls subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantToolCallsSubscribeResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/tool_calls/subscribe/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant tool_calls subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantToolCallsSubscribeResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/tool_calls/subscribe/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant tool_calls subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandLogsAgentsCompletionsResponseMessagesAssistantVideoClearResponseSchema = z.object({
  count: z.number().int().min(0).max(18446744073709552e3)
}).meta({ title: "cli.command.logs.agents.completions.response.messages.assistant.video.clear.Response" });

// src/viewer/command/logs/agents/completions/response/messages/assistant/video/clear.ts
async function logsAgentsCompletionsResponseMessagesAssistantVideoClearExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/video/clear" }), z.union([CliErrorSchema, CliCommandLogsAgentsCompletionsResponseMessagesAssistantVideoClearResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant video clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantVideoClearExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/video/clear" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant video clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantVideoClearRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/video/clear/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant video clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantVideoClearRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/video/clear/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant video clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantVideoClearResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/video/clear/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant video clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantVideoClearResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/video/clear/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant video clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantVideoGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/video/get" }), z.union([CliErrorSchema, AgentCompletionsMessageVideoUrlSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant video get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantVideoGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/video/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant video get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantVideoGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/video/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant video get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantVideoGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/video/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant video get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantVideoGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/video/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant video get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantVideoGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/video/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant video get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantVideoSubscribeExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/video/subscribe" }), z.union([CliErrorSchema, AgentCompletionsMessageVideoUrlSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant video subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantVideoSubscribeExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/video/subscribe" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant video subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantVideoSubscribeRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/video/subscribe/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant video subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantVideoSubscribeRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/video/subscribe/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant video subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantVideoSubscribeResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/assistant/video/subscribe/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant video subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesAssistantVideoSubscribeResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/assistant/video/subscribe/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages assistant video subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolAudioGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/tool/audio/get" }), z.union([CliErrorSchema, AgentCompletionsMessageInputAudioSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool audio get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolAudioGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/tool/audio/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool audio get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolAudioGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/tool/audio/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool audio get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolAudioGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/tool/audio/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool audio get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolAudioGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/tool/audio/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool audio get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolAudioGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/tool/audio/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool audio get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandLogsAgentsCompletionsResponseMessagesToolClearResponseSchema = z.object({
  count: z.number().int().min(0).max(18446744073709552e3)
}).meta({ title: "cli.command.logs.agents.completions.response.messages.tool.clear.Response" });

// src/viewer/command/logs/agents/completions/response/messages/tool/clear.ts
async function logsAgentsCompletionsResponseMessagesToolClearExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/tool/clear" }), z.union([CliErrorSchema, CliCommandLogsAgentsCompletionsResponseMessagesToolClearResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolClearExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/tool/clear" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolClearRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/tool/clear/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolClearRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/tool/clear/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolClearResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/tool/clear/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolClearResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/tool/clear/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolFileGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/tool/file/get" }), z.union([CliErrorSchema, AgentCompletionsMessageFileSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool file get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolFileGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/tool/file/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool file get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolFileGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/tool/file/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool file get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolFileGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/tool/file/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool file get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolFileGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/tool/file/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool file get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolFileGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/tool/file/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool file get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/tool/get" }), z.union([CliErrorSchema, AgentCompletionsResponseToolResponseLogSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/tool/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/tool/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/tool/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/tool/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/tool/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolImageGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/tool/image/get" }), z.union([CliErrorSchema, AgentCompletionsMessageImageUrlSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool image get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolImageGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/tool/image/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool image get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolImageGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/tool/image/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool image get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolImageGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/tool/image/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool image get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolImageGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/tool/image/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool image get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolImageGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/tool/image/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool image get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolSubscribeExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/tool/subscribe" }), z.union([CliErrorSchema, AgentCompletionsResponseToolResponseLogSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolSubscribeExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/tool/subscribe" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolSubscribeRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/tool/subscribe/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolSubscribeRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/tool/subscribe/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolSubscribeResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/tool/subscribe/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolSubscribeResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/tool/subscribe/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolTextGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/tool/text/get" }), z.union([CliErrorSchema, z.string()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool text get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolTextGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/tool/text/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool text get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolTextGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/tool/text/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool text get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolTextGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/tool/text/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool text get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolTextGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/tool/text/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool text get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolTextGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/tool/text/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool text get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolVideoGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/tool/video/get" }), z.union([CliErrorSchema, AgentCompletionsMessageVideoUrlSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool video get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolVideoGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/tool/video/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool video get: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolVideoGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/tool/video/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool video get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolVideoGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/tool/video/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool video get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolVideoGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/messages/tool/video/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool video get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseMessagesToolVideoGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/messages/tool/video/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response messages tool video get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseSubscribeExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/subscribe" }), z.union([CliErrorSchema, AgentCompletionsResponseStreamingAgentCompletionChunkLogSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseSubscribeExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/subscribe" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseSubscribeRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/subscribe/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseSubscribeRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/subscribe/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseSubscribeResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/agents/completions/response/subscribe/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsAgentsCompletionsResponseSubscribeResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/agents/completions/response/subscribe/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs agents completions response subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandLogsClearResponseSchema = z.object({
  count: z.number().int().min(0).max(18446744073709552e3)
}).meta({ title: "cli.command.logs.clear.Response" });

// src/viewer/command/logs/clear.ts
async function logsClearExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/clear" }), z.union([CliErrorSchema, CliCommandLogsClearResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsClearExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/clear" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsClearRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/clear/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsClearRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/clear/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsClearResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/clear/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsClearResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/clear/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsRequestGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/request/get" }), z.union([CliErrorSchema, FunctionsExecutionsRequestFunctionExecutionCreateParamsLogSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions request get: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsRequestGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/request/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions request get: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsRequestGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/request/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions request get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsRequestGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/request/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions request get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsRequestGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/request/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions request get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsRequestGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/request/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions request get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsRequestSubscribeExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/request/subscribe" }), z.union([CliErrorSchema, FunctionsExecutionsRequestFunctionExecutionCreateParamsLogSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions request subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsRequestSubscribeExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/request/subscribe" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions request subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsRequestSubscribeRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/request/subscribe/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions request subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsRequestSubscribeRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/request/subscribe/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions request subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsRequestSubscribeResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/request/subscribe/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions request subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsRequestSubscribeResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/request/subscribe/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions request subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandLogsFunctionsExecutionsResponseClearResponseSchema = z.object({
  count: z.number().int().min(0).max(18446744073709552e3)
}).meta({ title: "cli.command.logs.functions.executions.response.clear.Response" });

// src/viewer/command/logs/functions/executions/response/clear.ts
async function logsFunctionsExecutionsResponseClearExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/response/clear" }), z.union([CliErrorSchema, CliCommandLogsFunctionsExecutionsResponseClearResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseClearExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/response/clear" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseClearRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/response/clear/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseClearRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/response/clear/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseClearResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/response/clear/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseClearResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/response/clear/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/response/get" }), z.union([CliErrorSchema, FunctionsExecutionsResponseStreamingFunctionExecutionChunkLogSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response get: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/response/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response get: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/response/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/response/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/response/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/response/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandLogsFunctionsExecutionsResponseListResponseItemSchema = z.object({
  created: z.number().int().min(0).max(18446744073709552e3),
  id: z.string()
}).meta({ title: "cli.command.logs.functions.executions.response.list.ResponseItem" });

// src/viewer/command/logs/functions/executions/response/list.ts
function logsFunctionsExecutionsResponseListExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/response/list" }), z.union([CliErrorSchema, CliCommandLogsFunctionsExecutionsResponseListResponseItemSchema]));
}
function logsFunctionsExecutionsResponseListExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/response/list" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function logsFunctionsExecutionsResponseListRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/response/list/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseListRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/response/list/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseListResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/response/list/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response list response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseListResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/response/list/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response list response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandLogsFunctionsExecutionsResponseRetryTokensClearResponseSchema = z.object({
  count: z.number().int().min(0).max(18446744073709552e3)
}).meta({ title: "cli.command.logs.functions.executions.response.retry_tokens.clear.Response" });

// src/viewer/command/logs/functions/executions/response/retry_tokens/clear.ts
async function logsFunctionsExecutionsResponseRetryTokensClearExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/response/retry_tokens/clear" }), z.union([CliErrorSchema, CliCommandLogsFunctionsExecutionsResponseRetryTokensClearResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response retry_tokens clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseRetryTokensClearExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/response/retry_tokens/clear" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response retry_tokens clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseRetryTokensClearRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/response/retry_tokens/clear/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response retry_tokens clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseRetryTokensClearRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/response/retry_tokens/clear/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response retry_tokens clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseRetryTokensClearResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/response/retry_tokens/clear/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response retry_tokens clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseRetryTokensClearResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/response/retry_tokens/clear/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response retry_tokens clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseRetryTokensGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/response/retry_tokens/get" }), z.union([CliErrorSchema, z.string()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response retry_tokens get: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseRetryTokensGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/response/retry_tokens/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response retry_tokens get: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseRetryTokensGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/response/retry_tokens/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response retry_tokens get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseRetryTokensGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/response/retry_tokens/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response retry_tokens get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseRetryTokensGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/response/retry_tokens/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response retry_tokens get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseRetryTokensGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/response/retry_tokens/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response retry_tokens get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseRetryTokensSubscribeExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/response/retry_tokens/subscribe" }), z.union([CliErrorSchema, z.string()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response retry_tokens subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseRetryTokensSubscribeExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/response/retry_tokens/subscribe" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response retry_tokens subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseRetryTokensSubscribeRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/response/retry_tokens/subscribe/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response retry_tokens subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseRetryTokensSubscribeRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/response/retry_tokens/subscribe/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response retry_tokens subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseRetryTokensSubscribeResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/response/retry_tokens/subscribe/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response retry_tokens subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseRetryTokensSubscribeResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/response/retry_tokens/subscribe/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response retry_tokens subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseSubscribeExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/response/subscribe" }), z.union([CliErrorSchema, FunctionsExecutionsResponseStreamingFunctionExecutionChunkLogSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseSubscribeExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/response/subscribe" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseSubscribeRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/response/subscribe/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseSubscribeRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/response/subscribe/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseSubscribeResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/executions/response/subscribe/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsExecutionsResponseSubscribeResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/executions/response/subscribe/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions executions response subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveRequestGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/recursive/request/get" }), z.union([CliErrorSchema, FunctionsInventionsRecursiveRequestFunctionInventionRecursiveCreateParamsLogSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive request get: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveRequestGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/recursive/request/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive request get: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveRequestGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/recursive/request/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive request get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveRequestGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/recursive/request/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive request get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveRequestGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/recursive/request/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive request get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveRequestGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/recursive/request/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive request get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveRequestSubscribeExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/recursive/request/subscribe" }), z.union([CliErrorSchema, FunctionsInventionsRecursiveRequestFunctionInventionRecursiveCreateParamsLogSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive request subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveRequestSubscribeExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/recursive/request/subscribe" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive request subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveRequestSubscribeRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/recursive/request/subscribe/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive request subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveRequestSubscribeRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/recursive/request/subscribe/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive request subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveRequestSubscribeResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/recursive/request/subscribe/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive request subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveRequestSubscribeResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/recursive/request/subscribe/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive request subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandLogsFunctionsInventionsRecursiveResponseClearResponseSchema = z.object({
  count: z.number().int().min(0).max(18446744073709552e3)
}).meta({ title: "cli.command.logs.functions.inventions.recursive.response.clear.Response" });

// src/viewer/command/logs/functions/inventions/recursive/response/clear.ts
async function logsFunctionsInventionsRecursiveResponseClearExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/recursive/response/clear" }), z.union([CliErrorSchema, CliCommandLogsFunctionsInventionsRecursiveResponseClearResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive response clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveResponseClearExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/recursive/response/clear" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive response clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveResponseClearRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/recursive/response/clear/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive response clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveResponseClearRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/recursive/response/clear/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive response clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveResponseClearResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/recursive/response/clear/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive response clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveResponseClearResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/recursive/response/clear/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive response clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveResponseGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/recursive/response/get" }), z.union([CliErrorSchema, FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunkLogSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive response get: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveResponseGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/recursive/response/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive response get: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveResponseGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/recursive/response/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive response get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveResponseGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/recursive/response/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive response get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveResponseGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/recursive/response/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive response get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveResponseGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/recursive/response/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive response get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandLogsFunctionsInventionsRecursiveResponseListResponseItemSchema = z.object({
  created: z.number().int().min(0).max(18446744073709552e3),
  id: z.string()
}).meta({ title: "cli.command.logs.functions.inventions.recursive.response.list.ResponseItem" });

// src/viewer/command/logs/functions/inventions/recursive/response/list.ts
function logsFunctionsInventionsRecursiveResponseListExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/recursive/response/list" }), z.union([CliErrorSchema, CliCommandLogsFunctionsInventionsRecursiveResponseListResponseItemSchema]));
}
function logsFunctionsInventionsRecursiveResponseListExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/recursive/response/list" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function logsFunctionsInventionsRecursiveResponseListRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/recursive/response/list/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive response list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveResponseListRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/recursive/response/list/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive response list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveResponseListResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/recursive/response/list/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive response list response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveResponseListResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/recursive/response/list/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive response list response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveResponseSubscribeExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/recursive/response/subscribe" }), z.union([CliErrorSchema, FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunkLogSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive response subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveResponseSubscribeExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/recursive/response/subscribe" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive response subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveResponseSubscribeRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/recursive/response/subscribe/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive response subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveResponseSubscribeRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/recursive/response/subscribe/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive response subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveResponseSubscribeResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/recursive/response/subscribe/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive response subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRecursiveResponseSubscribeResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/recursive/response/subscribe/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions recursive response subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRequestGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/request/get" }), z.union([CliErrorSchema, FunctionsInventionsRequestFunctionInventionCreateParamsSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions request get: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRequestGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/request/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions request get: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRequestGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/request/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions request get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRequestGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/request/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions request get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRequestGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/request/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions request get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRequestGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/request/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions request get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRequestSubscribeExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/request/subscribe" }), z.union([CliErrorSchema, FunctionsInventionsRequestFunctionInventionCreateParamsSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions request subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRequestSubscribeExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/request/subscribe" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions request subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRequestSubscribeRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/request/subscribe/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions request subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRequestSubscribeRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/request/subscribe/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions request subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRequestSubscribeResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/request/subscribe/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions request subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsRequestSubscribeResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/request/subscribe/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions request subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandLogsFunctionsInventionsResponseClearResponseSchema = z.object({
  count: z.number().int().min(0).max(18446744073709552e3)
}).meta({ title: "cli.command.logs.functions.inventions.response.clear.Response" });

// src/viewer/command/logs/functions/inventions/response/clear.ts
async function logsFunctionsInventionsResponseClearExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/response/clear" }), z.union([CliErrorSchema, CliCommandLogsFunctionsInventionsResponseClearResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions response clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsResponseClearExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/response/clear" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions response clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsResponseClearRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/response/clear/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions response clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsResponseClearRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/response/clear/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions response clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsResponseClearResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/response/clear/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions response clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsResponseClearResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/response/clear/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions response clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsResponseGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/response/get" }), z.union([CliErrorSchema, FunctionsInventionsResponseStreamingFunctionInventionChunkLogSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions response get: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsResponseGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/response/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions response get: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsResponseGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/response/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions response get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsResponseGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/response/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions response get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsResponseGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/response/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions response get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsResponseGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/response/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions response get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandLogsFunctionsInventionsResponseListResponseItemSchema = z.object({
  created: z.number().int().min(0).max(18446744073709552e3),
  id: z.string()
}).meta({ title: "cli.command.logs.functions.inventions.response.list.ResponseItem" });

// src/viewer/command/logs/functions/inventions/response/list.ts
function logsFunctionsInventionsResponseListExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/response/list" }), z.union([CliErrorSchema, CliCommandLogsFunctionsInventionsResponseListResponseItemSchema]));
}
function logsFunctionsInventionsResponseListExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/response/list" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function logsFunctionsInventionsResponseListRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/response/list/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions response list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsResponseListRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/response/list/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions response list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsResponseListResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/response/list/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions response list response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsResponseListResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/response/list/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions response list response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsResponseSubscribeExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/response/subscribe" }), z.union([CliErrorSchema, FunctionsInventionsResponseStreamingFunctionInventionChunkLogSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions response subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsResponseSubscribeExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/response/subscribe" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions response subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsResponseSubscribeRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/response/subscribe/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions response subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsResponseSubscribeRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/response/subscribe/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions response subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsResponseSubscribeResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/functions/inventions/response/subscribe/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions response subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsFunctionsInventionsResponseSubscribeResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/functions/inventions/response/subscribe/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs functions inventions response subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsRequestGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/vector/completions/request/get" }), z.union([CliErrorSchema, VectorCompletionsRequestVectorCompletionCreateParamsSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions request get: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsRequestGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/vector/completions/request/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions request get: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsRequestGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/vector/completions/request/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions request get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsRequestGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/vector/completions/request/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions request get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsRequestGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/vector/completions/request/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions request get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsRequestGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/vector/completions/request/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions request get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsRequestSubscribeExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/vector/completions/request/subscribe" }), z.union([CliErrorSchema, VectorCompletionsRequestVectorCompletionCreateParamsSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions request subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsRequestSubscribeExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/vector/completions/request/subscribe" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions request subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsRequestSubscribeRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/vector/completions/request/subscribe/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions request subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsRequestSubscribeRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/vector/completions/request/subscribe/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions request subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsRequestSubscribeResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/vector/completions/request/subscribe/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions request subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsRequestSubscribeResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/vector/completions/request/subscribe/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions request subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandLogsVectorCompletionsResponseClearResponseSchema = z.object({
  count: z.number().int().min(0).max(18446744073709552e3)
}).meta({ title: "cli.command.logs.vector.completions.response.clear.Response" });

// src/viewer/command/logs/vector/completions/response/clear.ts
async function logsVectorCompletionsResponseClearExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/vector/completions/response/clear" }), z.union([CliErrorSchema, CliCommandLogsVectorCompletionsResponseClearResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions response clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsResponseClearExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/vector/completions/response/clear" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions response clear: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsResponseClearRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/vector/completions/response/clear/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions response clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsResponseClearRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/vector/completions/response/clear/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions response clear request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsResponseClearResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/vector/completions/response/clear/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions response clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsResponseClearResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/vector/completions/response/clear/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions response clear response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsResponseGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/vector/completions/response/get" }), z.union([CliErrorSchema, VectorCompletionsResponseStreamingVectorCompletionChunkLogSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions response get: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsResponseGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/vector/completions/response/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions response get: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsResponseGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/vector/completions/response/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions response get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsResponseGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/vector/completions/response/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions response get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsResponseGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/vector/completions/response/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions response get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsResponseGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/vector/completions/response/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions response get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandLogsVectorCompletionsResponseListResponseItemSchema = z.object({
  created: z.number().int().min(0).max(18446744073709552e3),
  id: z.string()
}).meta({ title: "cli.command.logs.vector.completions.response.list.ResponseItem" });

// src/viewer/command/logs/vector/completions/response/list.ts
function logsVectorCompletionsResponseListExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/vector/completions/response/list" }), z.union([CliErrorSchema, CliCommandLogsVectorCompletionsResponseListResponseItemSchema]));
}
function logsVectorCompletionsResponseListExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/vector/completions/response/list" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function logsVectorCompletionsResponseListRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/vector/completions/response/list/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions response list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsResponseListRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/vector/completions/response/list/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions response list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsResponseListResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/vector/completions/response/list/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions response list response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsResponseListResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/vector/completions/response/list/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions response list response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsResponseSubscribeExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/vector/completions/response/subscribe" }), z.union([CliErrorSchema, VectorCompletionsResponseStreamingVectorCompletionChunkLogSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions response subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsResponseSubscribeExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/vector/completions/response/subscribe" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions response subscribe: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsResponseSubscribeRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/vector/completions/response/subscribe/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions response subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsResponseSubscribeRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/vector/completions/response/subscribe/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions response subscribe request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsResponseSubscribeResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "logs/vector/completions/response/subscribe/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions response subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function logsVectorCompletionsResponseSubscribeResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "logs/vector/completions/response/subscribe/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("logs vector completions response subscribe response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandMcpKillResponseSchema = z.object({
  killed: z.number().int().min(0).max(4294967295)
}).meta({ title: "cli.command.mcp.kill.Response" });

// src/viewer/command/mcp/kill.ts
async function mcpKillExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "mcp/kill" }), z.union([CliErrorSchema, CliCommandMcpKillResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp kill: cli produced no output before the end marker");
  }
  return first;
}
async function mcpKillExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "mcp/kill" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp kill: cli produced no output before the end marker");
  }
  return first;
}
async function mcpKillRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "mcp/kill/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp kill request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpKillRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "mcp/kill/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp kill request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpKillResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "mcp/kill/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp kill response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpKillResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "mcp/kill/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
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
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "mcp/spawn" }), z.union([CliErrorSchema, CliCommandMcpSpawnResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp spawn: cli produced no output before the end marker");
  }
  return first;
}
async function mcpSpawnExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "mcp/spawn" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp spawn: cli produced no output before the end marker");
  }
  return first;
}
async function mcpSpawnRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "mcp/spawn/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp spawn request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpSpawnRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "mcp/spawn/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp spawn request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpSpawnResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "mcp/spawn/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp spawn response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function mcpSpawnResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "mcp/spawn/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("mcp spawn response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandPluginsGetResponseBinariesSchema = z.object({
  linux_aarch64: z.string().nullable().meta({ omitempty: true }).optional(),
  linux_x86_64: z.string().nullable().meta({ omitempty: true }).optional(),
  macos_aarch64: z.string().nullable().meta({ omitempty: true }).optional(),
  macos_x86_64: z.string().nullable().meta({ omitempty: true }).optional(),
  windows_aarch64: z.string().nullable().meta({ omitempty: true }).optional(),
  windows_x86_64: z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.plugins.get.ResponseBinaries" });
var CliCommandPluginsGetResponseMcpServerSchema = z.object({
  authorization: z.boolean(),
  name: z.string(),
  url: z.string()
}).meta({ title: "cli.command.plugins.get.ResponseMcpServer" });
var CliCommandPluginsGetResponseHttpMethodSchema = z.enum(["GET", "POST", "PUT", "PATCH", "DELETE"]).meta({ title: "cli.command.plugins.get.ResponseHttpMethod" });

// src/cli/command/plugins/get/responseViewerRoute.ts
var CliCommandPluginsGetResponseViewerRouteSchema = z.object({
  method: CliCommandPluginsGetResponseHttpMethodSchema,
  path: z.string(),
  type: z.string()
}).meta({ title: "cli.command.plugins.get.ResponseViewerRoute" });

// src/cli/command/plugins/get/responseManifest.ts
var CliCommandPluginsGetResponseManifestSchema = z.object({
  author: z.string().nullable().meta({ omitempty: true }).optional(),
  binaries: CliCommandPluginsGetResponseBinariesSchema,
  description: z.string(),
  homepage: z.string().nullable().meta({ omitempty: true }).optional(),
  license: z.string().nullable().meta({ omitempty: true }).optional(),
  mcp_servers: z.array(CliCommandPluginsGetResponseMcpServerSchema),
  mobile_ready: z.boolean(),
  name: z.string(),
  owner: z.string(),
  source: z.string(),
  version: z.string(),
  viewer_routes: z.array(CliCommandPluginsGetResponseViewerRouteSchema),
  viewer_url: z.string().nullable().meta({ omitempty: true }).optional(),
  viewer_zip: z.string().nullable().meta({ omitempty: true }).optional()
}).meta({ title: "cli.command.plugins.get.ResponseManifest" });

// src/viewer/command/plugins/get.ts
async function pluginsGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "plugins/get" }), z.union([CliErrorSchema, CliCommandPluginsGetResponseManifestSchema.nullable()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins get: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "plugins/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins get: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "plugins/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "plugins/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "plugins/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "plugins/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
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
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "plugins/install/filesystem" }), z.union([CliErrorSchema, CliCommandPluginsInstallFilesystemResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins install filesystem: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsInstallFilesystemExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "plugins/install/filesystem" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins install filesystem: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsInstallFilesystemRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "plugins/install/filesystem/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins install filesystem request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsInstallFilesystemRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "plugins/install/filesystem/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins install filesystem request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsInstallFilesystemResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "plugins/install/filesystem/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins install filesystem response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsInstallFilesystemResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "plugins/install/filesystem/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
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
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "plugins/install/github" }), z.union([CliErrorSchema, CliCommandPluginsInstallGithubResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins install github: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsInstallGithubExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "plugins/install/github" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins install github: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsInstallGithubRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "plugins/install/github/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins install github request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsInstallGithubRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "plugins/install/github/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins install github request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsInstallGithubResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "plugins/install/github/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins install github response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsInstallGithubResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "plugins/install/github/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins install github response_schema: cli produced no output before the end marker");
  }
  return first;
}
function pluginsListExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "plugins/list" }), z.union([CliErrorSchema, CliCommandPluginsGetResponseManifestSchema]));
}
function pluginsListExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "plugins/list" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function pluginsListRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "plugins/list/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsListRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "plugins/list/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsListResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "plugins/list/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins list response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsListResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "plugins/list/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
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
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "plugins/run" }), z.union([CliErrorSchema, CliCommandPluginsRunResponseItemSchema]));
}
function pluginsRunExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "plugins/run" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function pluginsRunRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "plugins/run/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins run request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsRunRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "plugins/run/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins run request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsRunResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "plugins/run/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins run response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function pluginsRunResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "plugins/run/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("plugins run response_schema: cli produced no output before the end marker");
  }
  return first;
}
var SwarmGetSwarmResponseSchema = RemotePathSchema.and(z.object({})).describe("Response containing a single Swarm.").meta({ title: "swarm.GetSwarmResponse" });

// src/viewer/command/swarms/get.ts
async function swarmsGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "swarms/get" }), z.union([CliErrorSchema, SwarmGetSwarmResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms get: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "swarms/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms get: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "swarms/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "swarms/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "swarms/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "swarms/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandSwarmsListResponseFavoriteSchema = z.union([z.object({
  commit: z.string().nullable().optional(),
  name: z.string(),
  note: z.string(),
  owner: z.string(),
  remote: z.literal("github"),
  repository: z.string()
}).meta({ "variantTitle": "Github" }), z.object({
  commit: z.string().nullable().optional(),
  name: z.string(),
  note: z.string(),
  owner: z.string(),
  remote: z.literal("filesystem"),
  repository: z.string()
}).meta({ "variantTitle": "Filesystem" }), z.object({
  name: z.string(),
  note: z.string(),
  remote: z.literal("mock")
}).meta({ "variantTitle": "Mock" })]).meta({ title: "cli.command.swarms.list.ResponseFavorite" });

// src/cli/command/swarms/list/responseItem.ts
var CliCommandSwarmsListResponseItemSchema = z.union([CliCommandSwarmsListResponseFavoriteSchema.meta({ "title": "cli.command.swarms.list.ResponseFavorite", "variantTitle": "Favorite" }), RemotePathSchema.meta({ "title": "RemotePath", "variantTitle": "Item" })]).meta({ title: "cli.command.swarms.list.ResponseItem" });

// src/viewer/command/swarms/list.ts
function swarmsListExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "swarms/list" }), z.union([CliErrorSchema, CliCommandSwarmsListResponseItemSchema]));
}
function swarmsListExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "swarms/list" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function swarmsListRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "swarms/list/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsListRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "swarms/list/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsListResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "swarms/list/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms list response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsListResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "swarms/list/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
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
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "swarms/publish" }), z.union([CliErrorSchema, CliCommandSwarmsPublishResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms publish: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsPublishExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "swarms/publish" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms publish: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsPublishRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "swarms/publish/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms publish request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsPublishRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "swarms/publish/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms publish request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsPublishResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "swarms/publish/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms publish response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function swarmsPublishResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "swarms/publish/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("swarms publish response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandToolsGetResponseManifestSchema = z.object({
  description: z.string(),
  exec: z.string(),
  name: z.string(),
  owner: z.string(),
  source: z.string(),
  version: z.string()
}).meta({ title: "cli.command.tools.get.ResponseManifest" });

// src/viewer/command/tools/get.ts
async function toolsGetExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "tools/get" }), z.union([CliErrorSchema, CliCommandToolsGetResponseManifestSchema.nullable()]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools get: cli produced no output before the end marker");
  }
  return first;
}
async function toolsGetExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "tools/get" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools get: cli produced no output before the end marker");
  }
  return first;
}
async function toolsGetRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "tools/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function toolsGetRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "tools/get/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools get request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function toolsGetResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "tools/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools get response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function toolsGetResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "tools/get/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools get response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandToolsInstallResponseSchema = z.object({
  instructions: z.string()
}).meta({ title: "cli.command.tools.install.Response" });

// src/viewer/command/tools/install.ts
async function toolsInstallExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "tools/install" }), z.union([CliErrorSchema, CliCommandToolsInstallResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools install: cli produced no output before the end marker");
  }
  return first;
}
async function toolsInstallExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "tools/install" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools install: cli produced no output before the end marker");
  }
  return first;
}
async function toolsInstallRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "tools/install/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools install request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function toolsInstallRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "tools/install/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools install request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function toolsInstallResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "tools/install/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools install response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function toolsInstallResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "tools/install/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools install response_schema: cli produced no output before the end marker");
  }
  return first;
}
function toolsListExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "tools/list" }), z.union([CliErrorSchema, CliCommandToolsGetResponseManifestSchema]));
}
function toolsListExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "tools/list" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function toolsListRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "tools/list/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function toolsListRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "tools/list/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools list request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function toolsListResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "tools/list/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools list response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function toolsListResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "tools/list/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools list response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandToolsRunResponseItemSchema = z.union([z.string().meta({ "variantTitle": "Stdout" }), CliErrorSchema.meta({ "title": "cli.Error", "variantTitle": "Stderr" })]).meta({ title: "cli.command.tools.run.ResponseItem" });

// src/viewer/command/tools/run.ts
function toolsRunExecute(request) {
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "tools/run" }), z.union([CliErrorSchema, CliCommandToolsRunResponseItemSchema]));
}
function toolsRunExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "tools/run" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function toolsRunRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "tools/run/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools run request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function toolsRunRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "tools/run/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools run request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function toolsRunResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "tools/run/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("tools run response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function toolsRunResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "tools/run/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
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
  return new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "update" }), z.union([CliErrorSchema, CliCommandUpdateResponseItemSchema]));
}
function updateExecuteJq(request, jq) {
  return new CliStream(invokeCliRequest({ ...request, jq, path_type: "update" }), z.union([CliErrorSchema, JsonValueSchema]));
}
async function updateRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "update/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("update request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function updateRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "update/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("update request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function updateResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "update/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("update response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function updateResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "update/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("update response_schema: cli produced no output before the end marker");
  }
  return first;
}
var CliCommandViewerGenerateSecretSignaturePairResponseSchema = z.object({
  secret: z.string(),
  signature: z.string()
}).meta({ title: "cli.command.viewer.generate_secret_signature_pair.Response" });

// src/viewer/command/viewer/generate_secret_signature_pair.ts
async function viewerGenerateSecretSignaturePairExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "viewer/generate_secret_signature_pair" }), z.union([CliErrorSchema, CliCommandViewerGenerateSecretSignaturePairResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer generate_secret_signature_pair: cli produced no output before the end marker");
  }
  return first;
}
async function viewerGenerateSecretSignaturePairExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "viewer/generate_secret_signature_pair" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer generate_secret_signature_pair: cli produced no output before the end marker");
  }
  return first;
}
async function viewerGenerateSecretSignaturePairRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "viewer/generate_secret_signature_pair/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer generate_secret_signature_pair request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerGenerateSecretSignaturePairRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "viewer/generate_secret_signature_pair/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer generate_secret_signature_pair request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerGenerateSecretSignaturePairResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "viewer/generate_secret_signature_pair/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer generate_secret_signature_pair response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerGenerateSecretSignaturePairResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "viewer/generate_secret_signature_pair/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
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
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "viewer/kill" }), z.union([CliErrorSchema, CliCommandViewerKillResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer kill: cli produced no output before the end marker");
  }
  return first;
}
async function viewerKillExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "viewer/kill" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer kill: cli produced no output before the end marker");
  }
  return first;
}
async function viewerKillRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "viewer/kill/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer kill request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerKillRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "viewer/kill/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer kill request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerKillResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "viewer/kill/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer kill response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerKillResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "viewer/kill/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
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
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "viewer/send" }), z.union([CliErrorSchema, CliCommandViewerSendResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer send: cli produced no output before the end marker");
  }
  return first;
}
async function viewerSendExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "viewer/send" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer send: cli produced no output before the end marker");
  }
  return first;
}
async function viewerSendRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "viewer/send/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer send request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerSendRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "viewer/send/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer send request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerSendResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "viewer/send/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer send response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerSendResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "viewer/send/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
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
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "viewer/spawn" }), z.union([CliErrorSchema, CliCommandViewerSpawnResponseSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer spawn: cli produced no output before the end marker");
  }
  return first;
}
async function viewerSpawnExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "viewer/spawn" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer spawn: cli produced no output before the end marker");
  }
  return first;
}
async function viewerSpawnRequestSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "viewer/spawn/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer spawn request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerSpawnRequestSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "viewer/spawn/request_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer spawn request_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerSpawnResponseSchemaExecute(request) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq: void 0, path_type: "viewer/spawn/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
  const first = await stream.first();
  if (first === void 0) {
    throw new Error("viewer spawn response_schema: cli produced no output before the end marker");
  }
  return first;
}
async function viewerSpawnResponseSchemaExecuteJq(request, jq) {
  const stream = new CliStream(invokeCliRequest({ ...request, jq, path_type: "viewer/spawn/response_schema" }), z.union([CliErrorSchema, JsonValueSchema]));
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

export { AgentClaudeAgentSdkAgentBaseSchema, AgentClaudeAgentSdkEffortSchema, AgentClaudeAgentSdkOutputModeSchema, AgentClaudeAgentSdkUpstreamSchema, AgentClientObjectiveaiMcpEntrySchema, AgentClientObjectiveaiMcpPluginEntrySchema, AgentClientObjectiveaiMcpPluginMcpServerSchema, AgentClientObjectiveaiMcpSchema, AgentCodexSdkAgentBaseSchema, AgentCodexSdkEffortSchema, AgentCodexSdkOutputModeSchema, AgentCodexSdkUpstreamSchema, AgentCompletionsMessageAssistantMessageExpressionSchema, AgentCompletionsMessageAssistantMessageLogSchema, AgentCompletionsMessageAssistantMessageSchema, AgentCompletionsMessageAssistantToolCallDeltaSchema, AgentCompletionsMessageAssistantToolCallExpressionSchema, AgentCompletionsMessageAssistantToolCallFunctionDeltaSchema, AgentCompletionsMessageAssistantToolCallFunctionExpressionSchema, AgentCompletionsMessageAssistantToolCallFunctionSchema, AgentCompletionsMessageAssistantToolCallSchema, AgentCompletionsMessageAssistantToolCallTypeSchema, AgentCompletionsMessageDeveloperMessageExpressionSchema, AgentCompletionsMessageDeveloperMessageLogSchema, AgentCompletionsMessageDeveloperMessageSchema, AgentCompletionsMessageFileSchema, AgentCompletionsMessageImageUrlDetailSchema, AgentCompletionsMessageImageUrlSchema, AgentCompletionsMessageInputAudioSchema, AgentCompletionsMessageMessageExpressionSchema, AgentCompletionsMessageMessageLogSchema, AgentCompletionsMessageMessageSchema, AgentCompletionsMessageRichContentExpressionSchema, AgentCompletionsMessageRichContentLogSchema, AgentCompletionsMessageRichContentPartExpressionSchema, AgentCompletionsMessageRichContentPartSchema, AgentCompletionsMessageRichContentSchema, AgentCompletionsMessageSimpleContentExpressionSchema, AgentCompletionsMessageSimpleContentLogSchema, AgentCompletionsMessageSimpleContentPartExpressionSchema, AgentCompletionsMessageSimpleContentPartSchema, AgentCompletionsMessageSimpleContentSchema, AgentCompletionsMessageSystemMessageExpressionSchema, AgentCompletionsMessageSystemMessageLogSchema, AgentCompletionsMessageSystemMessageSchema, AgentCompletionsMessageToolMessageExpressionSchema, AgentCompletionsMessageToolMessageLogSchema, AgentCompletionsMessageToolMessageSchema, AgentCompletionsMessageToolResponseMetadataSchema, AgentCompletionsMessageUserMessageExpressionSchema, AgentCompletionsMessageUserMessageLogSchema, AgentCompletionsMessageUserMessageSchema, AgentCompletionsMessageVideoUrlSchema, AgentCompletionsRequestAgentCompletionCreateParamsLogSchema, AgentCompletionsRequestProviderDataCollectionSchema, AgentCompletionsRequestProviderMaxPriceSchema, AgentCompletionsRequestProviderSchema, AgentCompletionsRequestProviderSortSchema, AgentCompletionsResponseAssistantRoleSchema, AgentCompletionsResponseCompletionTokensDetailsSchema, AgentCompletionsResponseCostDetailsSchema, AgentCompletionsResponseFinishReasonSchema, AgentCompletionsResponseLogprobSchema, AgentCompletionsResponseLogprobsSchema, AgentCompletionsResponsePromptTokensDetailsSchema, AgentCompletionsResponseStreamingAgentCompletionChunkLogSchema, AgentCompletionsResponseStreamingAgentCompletionChunkSchema, AgentCompletionsResponseStreamingAssistantResponseChunkLogSchema, AgentCompletionsResponseStreamingAssistantResponseChunkSchema, AgentCompletionsResponseStreamingLogReferenceSchema, AgentCompletionsResponseStreamingMessageChunkSchema, AgentCompletionsResponseStreamingObjectSchema, AgentCompletionsResponseStreamingRoleSchema, AgentCompletionsResponseToolResponseLogSchema, AgentCompletionsResponseToolResponseSchema, AgentCompletionsResponseToolRoleSchema, AgentCompletionsResponseTopLogprobSchema, AgentCompletionsResponseUpstreamUsageSchema, AgentCompletionsResponseUsageSchema, AgentGetAgentResponseSchema, AgentInlineAgentBaseSchema, AgentInlineAgentBaseWithFallbacksOrRemoteCommitOptionalSchema, AgentInlineAgentBaseWithFallbacksOrRemoteSchema, AgentInlineAgentBaseWithFallbacksOrRemoteWithCountSchema, AgentInlineAgentBaseWithFallbacksSchema, AgentMcpServerSchema, AgentMockAgentBaseSchema, AgentMockCallSchema, AgentMockCallToolCallSchema, AgentMockModeSchema, AgentMockOutputModeSchema, AgentMockUpstreamSchema, AgentOpenrouterAgentBaseSchema, AgentOpenrouterContextCompressionSchema, AgentOpenrouterOutputModeSchema, AgentOpenrouterProviderQuantizationSchema, AgentOpenrouterProviderSchema, AgentOpenrouterReasoningEffortSchema, AgentOpenrouterReasoningSchema, AgentOpenrouterReasoningSummaryVerbositySchema, AgentOpenrouterStopSchema, AgentOpenrouterUpstreamSchema, AgentOpenrouterVerbositySchema, AgentUpstreamSchema, CliCommandAgentsListActiveResponseItemSchema, CliCommandAgentsListAvailableResponseFavoriteSchema, CliCommandAgentsListAvailableResponseItemSchema, CliCommandAgentsMeResponseSchema, CliCommandAgentsMessageQueueAddResponseSchema, CliCommandAgentsMessageQueueDeleteResponseSchema, CliCommandAgentsMessageQueueDeliverResponseItemSchema, CliCommandAgentsMessageQueueReadPendingResponseItemSchema, CliCommandAgentsMessageResponseItemSchema, CliCommandAgentsMessageResponseSchema, CliCommandAgentsPublishResponseSchema, CliCommandAgentsReadAllResponseContentSchema, CliCommandAgentsReadAllResponseItemSchema, CliCommandAgentsReadAllResponseQueueItemSchema, CliCommandAgentsReadAllResponseQueueMessageSchema, CliCommandAgentsReadIdResponseSchema, CliCommandAgentsReadPendingResponseItemSchema, CliCommandAgentsReadSubscribeResponseItemSchema, CliCommandAgentsSpawnResponseItemSchema, CliCommandAgentsTagsAddResponseSchema, CliCommandAgentsTasksListResponseItemSchema, CliCommandAgentsTasksRunResponseItemSchema, CliCommandAgentsTasksScheduleResponseSchema, CliCommandConfigAgentsFavoritesGetResponseItemSchema, CliCommandConfigAgentsGetResponseSchema, CliCommandConfigFunctionsFavoritesGetResponseItemSchema, CliCommandConfigFunctionsGetResponseSchema, CliCommandConfigFunctionsInventionsGetResponseSchema, CliCommandConfigFunctionsProfilesFavoritesGetResponseItemSchema, CliCommandConfigFunctionsProfilesGetResponseSchema, CliCommandConfigFunctionsProfilesPairsFavoritesGetResponseItemSchema, CliCommandConfigFunctionsProfilesPairsGetResponseSchema, CliCommandConfigMcpAddressGetResponseSchema, CliCommandConfigMcpGetResponseSchema, CliCommandConfigMcpPortGetResponseSchema, CliCommandConfigSwarmsFavoritesGetResponseItemSchema, CliCommandConfigSwarmsGetResponseSchema, CliCommandConfigViewerAddressGetResponseSchema, CliCommandConfigViewerGetResponseSchema, CliCommandConfigViewerPortGetResponseSchema, CliCommandConfigViewerSecretGetResponseSchema, CliCommandConfigViewerSignatureGetResponseSchema, CliCommandFunctionsExecutionsCreateStandardResponseItemSchema, CliCommandFunctionsExecutionsCreateSwissSystemResponseItemSchema, CliCommandFunctionsInventionsRecursiveCreateAlphaScalarResponseItemSchema, CliCommandFunctionsInventionsRecursiveCreateAlphaVectorResponseItemSchema, CliCommandFunctionsInventionsRecursiveCreateRemoteResponseItemSchema, CliCommandFunctionsListResponseFavoriteSchema, CliCommandFunctionsListResponseItemSchema, CliCommandFunctionsProfilesListResponseFavoriteSchema, CliCommandFunctionsProfilesListResponseItemSchema, CliCommandFunctionsProfilesPublishResponseSchema, CliCommandFunctionsPublishResponseSchema, CliCommandLogsAgentsCompletionsResponseClearResponseSchema, CliCommandLogsAgentsCompletionsResponseContinuationsClearResponseSchema, CliCommandLogsAgentsCompletionsResponseListResponseItemSchema, CliCommandLogsAgentsCompletionsResponseMessagesAssistantAudioClearResponseSchema, CliCommandLogsAgentsCompletionsResponseMessagesAssistantClearResponseSchema, CliCommandLogsAgentsCompletionsResponseMessagesAssistantFileClearResponseSchema, CliCommandLogsAgentsCompletionsResponseMessagesAssistantImageClearResponseSchema, CliCommandLogsAgentsCompletionsResponseMessagesAssistantLogprobsClearResponseSchema, CliCommandLogsAgentsCompletionsResponseMessagesAssistantReasoningClearResponseSchema, CliCommandLogsAgentsCompletionsResponseMessagesAssistantRefusalClearResponseSchema, CliCommandLogsAgentsCompletionsResponseMessagesAssistantToolCallsClearResponseSchema, CliCommandLogsAgentsCompletionsResponseMessagesAssistantVideoClearResponseSchema, CliCommandLogsAgentsCompletionsResponseMessagesToolClearResponseSchema, CliCommandLogsClearResponseSchema, CliCommandLogsFunctionsExecutionsResponseClearResponseSchema, CliCommandLogsFunctionsExecutionsResponseListResponseItemSchema, CliCommandLogsFunctionsExecutionsResponseRetryTokensClearResponseSchema, CliCommandLogsFunctionsInventionsRecursiveResponseClearResponseSchema, CliCommandLogsFunctionsInventionsRecursiveResponseListResponseItemSchema, CliCommandLogsFunctionsInventionsResponseClearResponseSchema, CliCommandLogsFunctionsInventionsResponseListResponseItemSchema, CliCommandLogsVectorCompletionsResponseClearResponseSchema, CliCommandLogsVectorCompletionsResponseListResponseItemSchema, CliCommandMcpKillResponseSchema, CliCommandMcpSpawnResponseSchema, CliCommandOkSchema, CliCommandPluginsGetResponseBinariesSchema, CliCommandPluginsGetResponseHttpMethodSchema, CliCommandPluginsGetResponseManifestSchema, CliCommandPluginsGetResponseMcpServerSchema, CliCommandPluginsGetResponseViewerRouteSchema, CliCommandPluginsInstallFilesystemResponseSchema, CliCommandPluginsInstallGithubResponseSchema, CliCommandPluginsRunMcpSchema, CliCommandPluginsRunMcpTypeSchema, CliCommandPluginsRunResponseItemSchema, CliCommandSwarmsListResponseFavoriteSchema, CliCommandSwarmsListResponseItemSchema, CliCommandSwarmsPublishResponseSchema, CliCommandToolsGetResponseManifestSchema, CliCommandToolsInstallResponseSchema, CliCommandToolsRunResponseItemSchema, CliCommandUpdateResponseItemSchema, CliCommandUpdateResponseSkipReasonSchema, CliCommandViewerGenerateSecretSignaturePairResponseSchema, CliCommandViewerKillResponseSchema, CliCommandViewerSendResponseSchema, CliCommandViewerSpawnResponseSchema, CliErrorSchema, CliErrorTypeSchema, CliLevelSchema, CliStream, ErrorResponseErrorSchema, FunctionsAlphaInlineFunctionSchema, FunctionsAlphaRemoteFunctionSchema, FunctionsAlphaScalarBranchTaskExpressionSchema, FunctionsAlphaScalarInlineFunctionSchema, FunctionsAlphaScalarLeafTaskExpressionSchema, FunctionsAlphaScalarPlaceholderScalarFunctionTaskExpressionSchema, FunctionsAlphaScalarRemoteFunctionSchema, FunctionsAlphaScalarScalarFunctionTaskExpressionSchema, FunctionsAlphaScalarVectorCompletionTaskExpressionSchema, FunctionsAlphaVectorBranchTaskExpressionSchema, FunctionsAlphaVectorExpressionVectorFunctionInputSchemaSchema, FunctionsAlphaVectorExpressionVectorFunctionInputValueExpressionSchema, FunctionsAlphaVectorInlineFunctionSchema, FunctionsAlphaVectorLeafTaskExpressionSchema, FunctionsAlphaVectorPlaceholderScalarFunctionTaskExpressionSchema, FunctionsAlphaVectorPlaceholderVectorFunctionTaskExpressionSchema, FunctionsAlphaVectorRemoteFunctionSchema, FunctionsAlphaVectorScalarFunctionTaskExpressionSchema, FunctionsAlphaVectorVectorCompletionTaskExpressionSchema, FunctionsAlphaVectorVectorFunctionTaskExpressionSchema, FunctionsExecutionsRequestFunctionExecutionCreateParamsLogSchema, FunctionsExecutionsRequestReasoningSchema, FunctionsExecutionsRequestStrategySchema, FunctionsExecutionsResponseOutputSchema, FunctionsExecutionsResponseStreamingFunctionExecutionChunkLogSchema, FunctionsExecutionsResponseStreamingFunctionExecutionChunkSchema, FunctionsExecutionsResponseStreamingFunctionExecutionTaskChunkSchema, FunctionsExecutionsResponseStreamingFunctionExecutionTaskLogReferenceLogReferenceSchema, FunctionsExecutionsResponseStreamingObjectSchema, FunctionsExecutionsResponseStreamingReasoningSummaryChunkSchema, FunctionsExecutionsResponseStreamingReasoningSummaryLogReferenceLogReferenceSchema, FunctionsExecutionsResponseStreamingTaskChunkSchema, FunctionsExecutionsResponseStreamingTaskLogReferenceLogReferenceSchema, FunctionsExecutionsResponseStreamingVectorCompletionTaskChunkSchema, FunctionsExecutionsResponseStreamingVectorCompletionTaskLogReferenceLogReferenceSchema, FunctionsExpressionAnyOfInputSchemaSchema, FunctionsExpressionArrayInputSchemaSchema, FunctionsExpressionArrayInputSchemaTypeSchema, FunctionsExpressionAudioInputSchemaSchema, FunctionsExpressionAudioInputSchemaTypeSchema, FunctionsExpressionBooleanInputSchemaSchema, FunctionsExpressionBooleanInputSchemaTypeSchema, FunctionsExpressionExpressionSchema, FunctionsExpressionFileInputSchemaSchema, FunctionsExpressionFileInputSchemaTypeSchema, FunctionsExpressionImageInputSchemaSchema, FunctionsExpressionImageInputSchemaTypeSchema, FunctionsExpressionInputSchemaSchema, FunctionsExpressionInputValueExpressionSchema, FunctionsExpressionInputValueLogSchema, FunctionsExpressionIntegerInputSchemaSchema, FunctionsExpressionIntegerInputSchemaTypeSchema, FunctionsExpressionNumberInputSchemaSchema, FunctionsExpressionNumberInputSchemaTypeSchema, FunctionsExpressionObjectInputSchemaSchema, FunctionsExpressionObjectInputSchemaTypeSchema, FunctionsExpressionSpecialSchema, FunctionsExpressionStringInputSchemaSchema, FunctionsExpressionStringInputSchemaTypeSchema, FunctionsExpressionTaskOutputSchema, FunctionsExpressionVideoInputSchemaSchema, FunctionsExpressionVideoInputSchemaTypeSchema, FunctionsFullInlineFunctionOrRemoteCommitOptionalSchema, FunctionsFullInlineFunctionSchema, FunctionsFullRemoteFunctionSchema, FunctionsGetFunctionResponseSchema, FunctionsInlineFunctionSchema, FunctionsInlineProfileOrRemoteCommitOptionalSchema, FunctionsInlineProfileSchema, FunctionsInlineTasksProfileSchema, FunctionsInventionsPromptsInlinePromptOrRemoteCommitOptionalSchema, FunctionsInventionsPromptsInlinePromptSchema, FunctionsInventionsPromptsStepPromptExpressionSchema, FunctionsInventionsPromptsStepPromptTypeSchema, FunctionsInventionsRecursiveRequestFunctionInventionRecursiveCreateParamsLogSchema, FunctionsInventionsRecursiveResponseStreamingFunctionInventionChunkSchema, FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunkLogSchema, FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunkSchema, FunctionsInventionsRecursiveResponseStreamingObjectSchema, FunctionsInventionsRequestFunctionInventionCreateParamsSchema, FunctionsInventionsResponseStreamingAgentCompletionChunkSchema, FunctionsInventionsResponseStreamingFunctionInventionChunkLogSchema, FunctionsInventionsResponseStreamingObjectSchema, FunctionsInventionsStateAlphaScalarBranchStateSchema, FunctionsInventionsStateAlphaScalarLeafStateSchema, FunctionsInventionsStateAlphaScalarStateSchema, FunctionsInventionsStateAlphaVectorBranchStateSchema, FunctionsInventionsStateAlphaVectorLeafStateSchema, FunctionsInventionsStateAlphaVectorStateSchema, FunctionsInventionsStateGetFunctionInventionStateResponseSchema, FunctionsInventionsStateParamsStateOrRemoteCommitOptionalSchema, FunctionsInventionsStateParamsStateSchema, FunctionsInventionsStateStateSchema, FunctionsPlaceholderScalarFunctionTaskExpressionSchema, FunctionsPlaceholderVectorFunctionTaskExpressionSchema, FunctionsProfilesGetProfileResponseSchema, FunctionsRemoteFunctionSchema, FunctionsScalarFunctionTaskExpressionSchema, FunctionsTaskExpressionSchema, FunctionsTaskProfileSchema, FunctionsVectorCompletionTaskExpressionSchema, FunctionsVectorFunctionTaskExpressionSchema, IndexedLogReferenceSchema, JsonValueSchema, LogReferenceSchema, LogReferenceTagSchema, RemotePathCommitOptionalSchema, RemotePathSchema, RemoteSchema, SwarmGetSwarmResponseSchema, SwarmInlineSwarmBaseOrRemoteCommitOptionalSchema, SwarmInlineSwarmBaseSchema, VectorCompletionsRequestVectorCompletionCreateParamsSchema, VectorCompletionsResponseStreamingAgentCompletionChunkSchema, VectorCompletionsResponseStreamingObjectSchema, VectorCompletionsResponseStreamingVectorCompletionChunkLogSchema, VectorCompletionsResponseVoteSchema, ViewerEventSchema, WeightsEntrySchema, WeightsSchema, __resetForTests, agentsGetExecute, agentsGetExecuteJq, agentsGetRequestSchemaExecute, agentsGetRequestSchemaExecuteJq, agentsGetResponseSchemaExecute, agentsGetResponseSchemaExecuteJq, agentsListActiveExecute, agentsListActiveExecuteJq, agentsListActiveRequestSchemaExecute, agentsListActiveRequestSchemaExecuteJq, agentsListActiveResponseSchemaExecute, agentsListActiveResponseSchemaExecuteJq, agentsListAvailableExecute, agentsListAvailableExecuteJq, agentsListAvailableRequestSchemaExecute, agentsListAvailableRequestSchemaExecuteJq, agentsListAvailableResponseSchemaExecute, agentsListAvailableResponseSchemaExecuteJq, agentsMeExecute, agentsMeExecuteJq, agentsMeRequestSchemaExecute, agentsMeRequestSchemaExecuteJq, agentsMeResponseSchemaExecute, agentsMeResponseSchemaExecuteJq, agentsMessageExecute, agentsMessageExecuteJq, agentsMessageExecuteStreaming, agentsMessageExecuteStreamingJq, agentsMessageQueueAddExecute, agentsMessageQueueAddExecuteJq, agentsMessageQueueAddRequestSchemaExecute, agentsMessageQueueAddRequestSchemaExecuteJq, agentsMessageQueueAddResponseSchemaExecute, agentsMessageQueueAddResponseSchemaExecuteJq, agentsMessageQueueDeleteExecute, agentsMessageQueueDeleteExecuteJq, agentsMessageQueueDeleteRequestSchemaExecute, agentsMessageQueueDeleteRequestSchemaExecuteJq, agentsMessageQueueDeleteResponseSchemaExecute, agentsMessageQueueDeleteResponseSchemaExecuteJq, agentsMessageQueueDeliverExecute, agentsMessageQueueDeliverExecuteJq, agentsMessageQueueDeliverRequestSchemaExecute, agentsMessageQueueDeliverRequestSchemaExecuteJq, agentsMessageQueueDeliverResponseSchemaExecute, agentsMessageQueueDeliverResponseSchemaExecuteJq, agentsMessageQueueReadIdExecute, agentsMessageQueueReadIdExecuteJq, agentsMessageQueueReadIdRequestSchemaExecute, agentsMessageQueueReadIdRequestSchemaExecuteJq, agentsMessageQueueReadIdResponseSchemaExecute, agentsMessageQueueReadIdResponseSchemaExecuteJq, agentsMessageQueueReadPendingExecute, agentsMessageQueueReadPendingExecuteJq, agentsMessageQueueReadPendingRequestSchemaExecute, agentsMessageQueueReadPendingRequestSchemaExecuteJq, agentsMessageQueueReadPendingResponseSchemaExecute, agentsMessageQueueReadPendingResponseSchemaExecuteJq, agentsMessageRequestSchemaExecute, agentsMessageRequestSchemaExecuteJq, agentsMessageResponseSchemaExecute, agentsMessageResponseSchemaExecuteJq, agentsPublishExecute, agentsPublishExecuteJq, agentsPublishRequestSchemaExecute, agentsPublishRequestSchemaExecuteJq, agentsPublishResponseSchemaExecute, agentsPublishResponseSchemaExecuteJq, agentsReadAllExecute, agentsReadAllExecuteJq, agentsReadAllRequestSchemaExecute, agentsReadAllRequestSchemaExecuteJq, agentsReadAllResponseSchemaExecute, agentsReadAllResponseSchemaExecuteJq, agentsReadIdExecute, agentsReadIdExecuteJq, agentsReadIdRequestSchemaExecute, agentsReadIdRequestSchemaExecuteJq, agentsReadIdResponseSchemaExecute, agentsReadIdResponseSchemaExecuteJq, agentsReadPendingExecute, agentsReadPendingExecuteJq, agentsReadPendingRequestSchemaExecute, agentsReadPendingRequestSchemaExecuteJq, agentsReadPendingResponseSchemaExecute, agentsReadPendingResponseSchemaExecuteJq, agentsReadSubscribeExecute, agentsReadSubscribeExecuteJq, agentsReadSubscribeRequestSchemaExecute, agentsReadSubscribeRequestSchemaExecuteJq, agentsReadSubscribeResponseSchemaExecute, agentsReadSubscribeResponseSchemaExecuteJq, agentsSpawnExecute, agentsSpawnExecuteJq, agentsSpawnExecuteStreaming, agentsSpawnExecuteStreamingJq, agentsSpawnRequestSchemaExecute, agentsSpawnRequestSchemaExecuteJq, agentsSpawnResponseSchemaExecute, agentsSpawnResponseSchemaExecuteJq, agentsTagsAddExecute, agentsTagsAddExecuteJq, agentsTagsAddRequestSchemaExecute, agentsTagsAddRequestSchemaExecuteJq, agentsTagsAddResponseSchemaExecute, agentsTagsAddResponseSchemaExecuteJq, agentsTagsLookupRequestSchemaExecute, agentsTagsLookupRequestSchemaExecuteJq, agentsTagsLookupResponseSchemaExecute, agentsTagsLookupResponseSchemaExecuteJq, agentsTasksListExecute, agentsTasksListExecuteJq, agentsTasksListRequestSchemaExecute, agentsTasksListRequestSchemaExecuteJq, agentsTasksListResponseSchemaExecute, agentsTasksListResponseSchemaExecuteJq, agentsTasksRunExecute, agentsTasksRunExecuteJq, agentsTasksRunRequestSchemaExecute, agentsTasksRunRequestSchemaExecuteJq, agentsTasksRunResponseSchemaExecute, agentsTasksRunResponseSchemaExecuteJq, agentsTasksScheduleExecute, agentsTasksScheduleExecuteJq, agentsTasksScheduleRequestSchemaExecute, agentsTasksScheduleRequestSchemaExecuteJq, agentsTasksScheduleResponseSchemaExecute, agentsTasksScheduleResponseSchemaExecuteJq, configAgentsFavoritesAddExecute, configAgentsFavoritesAddExecuteJq, configAgentsFavoritesAddRequestSchemaExecute, configAgentsFavoritesAddRequestSchemaExecuteJq, configAgentsFavoritesAddResponseSchemaExecute, configAgentsFavoritesAddResponseSchemaExecuteJq, configAgentsFavoritesDelExecute, configAgentsFavoritesDelExecuteJq, configAgentsFavoritesDelRequestSchemaExecute, configAgentsFavoritesDelRequestSchemaExecuteJq, configAgentsFavoritesDelResponseSchemaExecute, configAgentsFavoritesDelResponseSchemaExecuteJq, configAgentsFavoritesEditExecute, configAgentsFavoritesEditExecuteJq, configAgentsFavoritesEditRequestSchemaExecute, configAgentsFavoritesEditRequestSchemaExecuteJq, configAgentsFavoritesEditResponseSchemaExecute, configAgentsFavoritesEditResponseSchemaExecuteJq, configAgentsFavoritesGetExecute, configAgentsFavoritesGetExecuteJq, configAgentsFavoritesGetRequestSchemaExecute, configAgentsFavoritesGetRequestSchemaExecuteJq, configAgentsFavoritesGetResponseSchemaExecute, configAgentsFavoritesGetResponseSchemaExecuteJq, configAgentsGetExecute, configAgentsGetExecuteJq, configAgentsGetRequestSchemaExecute, configAgentsGetRequestSchemaExecuteJq, configAgentsGetResponseSchemaExecute, configAgentsGetResponseSchemaExecuteJq, configFunctionsFavoritesAddExecute, configFunctionsFavoritesAddExecuteJq, configFunctionsFavoritesAddRequestSchemaExecute, configFunctionsFavoritesAddRequestSchemaExecuteJq, configFunctionsFavoritesAddResponseSchemaExecute, configFunctionsFavoritesAddResponseSchemaExecuteJq, configFunctionsFavoritesDelExecute, configFunctionsFavoritesDelExecuteJq, configFunctionsFavoritesDelRequestSchemaExecute, configFunctionsFavoritesDelRequestSchemaExecuteJq, configFunctionsFavoritesDelResponseSchemaExecute, configFunctionsFavoritesDelResponseSchemaExecuteJq, configFunctionsFavoritesEditExecute, configFunctionsFavoritesEditExecuteJq, configFunctionsFavoritesEditRequestSchemaExecute, configFunctionsFavoritesEditRequestSchemaExecuteJq, configFunctionsFavoritesEditResponseSchemaExecute, configFunctionsFavoritesEditResponseSchemaExecuteJq, configFunctionsFavoritesGetExecute, configFunctionsFavoritesGetExecuteJq, configFunctionsFavoritesGetRequestSchemaExecute, configFunctionsFavoritesGetRequestSchemaExecuteJq, configFunctionsFavoritesGetResponseSchemaExecute, configFunctionsFavoritesGetResponseSchemaExecuteJq, configFunctionsGetExecute, configFunctionsGetExecuteJq, configFunctionsGetRequestSchemaExecute, configFunctionsGetRequestSchemaExecuteJq, configFunctionsGetResponseSchemaExecute, configFunctionsGetResponseSchemaExecuteJq, configFunctionsInventionsGetExecute, configFunctionsInventionsGetExecuteJq, configFunctionsInventionsGetRequestSchemaExecute, configFunctionsInventionsGetRequestSchemaExecuteJq, configFunctionsInventionsGetResponseSchemaExecute, configFunctionsInventionsGetResponseSchemaExecuteJq, configFunctionsInventionsRemoteGetExecute, configFunctionsInventionsRemoteGetExecuteJq, configFunctionsInventionsRemoteGetRequestSchemaExecute, configFunctionsInventionsRemoteGetRequestSchemaExecuteJq, configFunctionsInventionsRemoteGetResponseSchemaExecute, configFunctionsInventionsRemoteGetResponseSchemaExecuteJq, configFunctionsInventionsRemoteSetExecute, configFunctionsInventionsRemoteSetExecuteJq, configFunctionsInventionsRemoteSetRequestSchemaExecute, configFunctionsInventionsRemoteSetRequestSchemaExecuteJq, configFunctionsInventionsRemoteSetResponseSchemaExecute, configFunctionsInventionsRemoteSetResponseSchemaExecuteJq, configFunctionsProfilesFavoritesAddExecute, configFunctionsProfilesFavoritesAddExecuteJq, configFunctionsProfilesFavoritesAddRequestSchemaExecute, configFunctionsProfilesFavoritesAddRequestSchemaExecuteJq, configFunctionsProfilesFavoritesAddResponseSchemaExecute, configFunctionsProfilesFavoritesAddResponseSchemaExecuteJq, configFunctionsProfilesFavoritesDelExecute, configFunctionsProfilesFavoritesDelExecuteJq, configFunctionsProfilesFavoritesDelRequestSchemaExecute, configFunctionsProfilesFavoritesDelRequestSchemaExecuteJq, configFunctionsProfilesFavoritesDelResponseSchemaExecute, configFunctionsProfilesFavoritesDelResponseSchemaExecuteJq, configFunctionsProfilesFavoritesEditExecute, configFunctionsProfilesFavoritesEditExecuteJq, configFunctionsProfilesFavoritesEditRequestSchemaExecute, configFunctionsProfilesFavoritesEditRequestSchemaExecuteJq, configFunctionsProfilesFavoritesEditResponseSchemaExecute, configFunctionsProfilesFavoritesEditResponseSchemaExecuteJq, configFunctionsProfilesFavoritesGetExecute, configFunctionsProfilesFavoritesGetExecuteJq, configFunctionsProfilesFavoritesGetRequestSchemaExecute, configFunctionsProfilesFavoritesGetRequestSchemaExecuteJq, configFunctionsProfilesFavoritesGetResponseSchemaExecute, configFunctionsProfilesFavoritesGetResponseSchemaExecuteJq, configFunctionsProfilesGetExecute, configFunctionsProfilesGetExecuteJq, configFunctionsProfilesGetRequestSchemaExecute, configFunctionsProfilesGetRequestSchemaExecuteJq, configFunctionsProfilesGetResponseSchemaExecute, configFunctionsProfilesGetResponseSchemaExecuteJq, configFunctionsProfilesPairsFavoritesAddExecute, configFunctionsProfilesPairsFavoritesAddExecuteJq, configFunctionsProfilesPairsFavoritesAddRequestSchemaExecute, configFunctionsProfilesPairsFavoritesAddRequestSchemaExecuteJq, configFunctionsProfilesPairsFavoritesAddResponseSchemaExecute, configFunctionsProfilesPairsFavoritesAddResponseSchemaExecuteJq, configFunctionsProfilesPairsFavoritesDelExecute, configFunctionsProfilesPairsFavoritesDelExecuteJq, configFunctionsProfilesPairsFavoritesDelRequestSchemaExecute, configFunctionsProfilesPairsFavoritesDelRequestSchemaExecuteJq, configFunctionsProfilesPairsFavoritesDelResponseSchemaExecute, configFunctionsProfilesPairsFavoritesDelResponseSchemaExecuteJq, configFunctionsProfilesPairsFavoritesEditExecute, configFunctionsProfilesPairsFavoritesEditExecuteJq, configFunctionsProfilesPairsFavoritesEditRequestSchemaExecute, configFunctionsProfilesPairsFavoritesEditRequestSchemaExecuteJq, configFunctionsProfilesPairsFavoritesEditResponseSchemaExecute, configFunctionsProfilesPairsFavoritesEditResponseSchemaExecuteJq, configFunctionsProfilesPairsFavoritesGetExecute, configFunctionsProfilesPairsFavoritesGetExecuteJq, configFunctionsProfilesPairsFavoritesGetRequestSchemaExecute, configFunctionsProfilesPairsFavoritesGetRequestSchemaExecuteJq, configFunctionsProfilesPairsFavoritesGetResponseSchemaExecute, configFunctionsProfilesPairsFavoritesGetResponseSchemaExecuteJq, configFunctionsProfilesPairsGetExecute, configFunctionsProfilesPairsGetExecuteJq, configFunctionsProfilesPairsGetRequestSchemaExecute, configFunctionsProfilesPairsGetRequestSchemaExecuteJq, configFunctionsProfilesPairsGetResponseSchemaExecute, configFunctionsProfilesPairsGetResponseSchemaExecuteJq, configMcpAddressGetExecute, configMcpAddressGetExecuteJq, configMcpAddressGetRequestSchemaExecute, configMcpAddressGetRequestSchemaExecuteJq, configMcpAddressGetResponseSchemaExecute, configMcpAddressGetResponseSchemaExecuteJq, configMcpAddressSetExecute, configMcpAddressSetExecuteJq, configMcpAddressSetRequestSchemaExecute, configMcpAddressSetRequestSchemaExecuteJq, configMcpAddressSetResponseSchemaExecute, configMcpAddressSetResponseSchemaExecuteJq, configMcpGetExecute, configMcpGetExecuteJq, configMcpGetRequestSchemaExecute, configMcpGetRequestSchemaExecuteJq, configMcpGetResponseSchemaExecute, configMcpGetResponseSchemaExecuteJq, configMcpPortGetExecute, configMcpPortGetExecuteJq, configMcpPortGetRequestSchemaExecute, configMcpPortGetRequestSchemaExecuteJq, configMcpPortGetResponseSchemaExecute, configMcpPortGetResponseSchemaExecuteJq, configMcpPortSetExecute, configMcpPortSetExecuteJq, configMcpPortSetRequestSchemaExecute, configMcpPortSetRequestSchemaExecuteJq, configMcpPortSetResponseSchemaExecute, configMcpPortSetResponseSchemaExecuteJq, configSwarmsFavoritesAddExecute, configSwarmsFavoritesAddExecuteJq, configSwarmsFavoritesAddRequestSchemaExecute, configSwarmsFavoritesAddRequestSchemaExecuteJq, configSwarmsFavoritesAddResponseSchemaExecute, configSwarmsFavoritesAddResponseSchemaExecuteJq, configSwarmsFavoritesDelExecute, configSwarmsFavoritesDelExecuteJq, configSwarmsFavoritesDelRequestSchemaExecute, configSwarmsFavoritesDelRequestSchemaExecuteJq, configSwarmsFavoritesDelResponseSchemaExecute, configSwarmsFavoritesDelResponseSchemaExecuteJq, configSwarmsFavoritesEditExecute, configSwarmsFavoritesEditExecuteJq, configSwarmsFavoritesEditRequestSchemaExecute, configSwarmsFavoritesEditRequestSchemaExecuteJq, configSwarmsFavoritesEditResponseSchemaExecute, configSwarmsFavoritesEditResponseSchemaExecuteJq, configSwarmsFavoritesGetExecute, configSwarmsFavoritesGetExecuteJq, configSwarmsFavoritesGetRequestSchemaExecute, configSwarmsFavoritesGetRequestSchemaExecuteJq, configSwarmsFavoritesGetResponseSchemaExecute, configSwarmsFavoritesGetResponseSchemaExecuteJq, configSwarmsGetExecute, configSwarmsGetExecuteJq, configSwarmsGetRequestSchemaExecute, configSwarmsGetRequestSchemaExecuteJq, configSwarmsGetResponseSchemaExecute, configSwarmsGetResponseSchemaExecuteJq, configViewerAddressGetExecute, configViewerAddressGetExecuteJq, configViewerAddressGetRequestSchemaExecute, configViewerAddressGetRequestSchemaExecuteJq, configViewerAddressGetResponseSchemaExecute, configViewerAddressGetResponseSchemaExecuteJq, configViewerAddressSetExecute, configViewerAddressSetExecuteJq, configViewerAddressSetRequestSchemaExecute, configViewerAddressSetRequestSchemaExecuteJq, configViewerAddressSetResponseSchemaExecute, configViewerAddressSetResponseSchemaExecuteJq, configViewerGetExecute, configViewerGetExecuteJq, configViewerGetRequestSchemaExecute, configViewerGetRequestSchemaExecuteJq, configViewerGetResponseSchemaExecute, configViewerGetResponseSchemaExecuteJq, configViewerPortGetExecute, configViewerPortGetExecuteJq, configViewerPortGetRequestSchemaExecute, configViewerPortGetRequestSchemaExecuteJq, configViewerPortGetResponseSchemaExecute, configViewerPortGetResponseSchemaExecuteJq, configViewerPortSetExecute, configViewerPortSetExecuteJq, configViewerPortSetRequestSchemaExecute, configViewerPortSetRequestSchemaExecuteJq, configViewerPortSetResponseSchemaExecute, configViewerPortSetResponseSchemaExecuteJq, configViewerSecretGetExecute, configViewerSecretGetExecuteJq, configViewerSecretGetRequestSchemaExecute, configViewerSecretGetRequestSchemaExecuteJq, configViewerSecretGetResponseSchemaExecute, configViewerSecretGetResponseSchemaExecuteJq, configViewerSecretSetExecute, configViewerSecretSetExecuteJq, configViewerSecretSetRequestSchemaExecute, configViewerSecretSetRequestSchemaExecuteJq, configViewerSecretSetResponseSchemaExecute, configViewerSecretSetResponseSchemaExecuteJq, configViewerSignatureGetExecute, configViewerSignatureGetExecuteJq, configViewerSignatureGetRequestSchemaExecute, configViewerSignatureGetRequestSchemaExecuteJq, configViewerSignatureGetResponseSchemaExecute, configViewerSignatureGetResponseSchemaExecuteJq, configViewerSignatureSetExecute, configViewerSignatureSetExecuteJq, configViewerSignatureSetRequestSchemaExecute, configViewerSignatureSetRequestSchemaExecuteJq, configViewerSignatureSetResponseSchemaExecute, configViewerSignatureSetResponseSchemaExecuteJq, functionsExecutionsCreateStandardExecute, functionsExecutionsCreateStandardExecuteJq, functionsExecutionsCreateStandardExecuteStreaming, functionsExecutionsCreateStandardExecuteStreamingJq, functionsExecutionsCreateStandardRequestSchemaExecute, functionsExecutionsCreateStandardRequestSchemaExecuteJq, functionsExecutionsCreateStandardResponseSchemaExecute, functionsExecutionsCreateStandardResponseSchemaExecuteJq, functionsExecutionsCreateSwissSystemExecute, functionsExecutionsCreateSwissSystemExecuteJq, functionsExecutionsCreateSwissSystemExecuteStreaming, functionsExecutionsCreateSwissSystemExecuteStreamingJq, functionsExecutionsCreateSwissSystemRequestSchemaExecute, functionsExecutionsCreateSwissSystemRequestSchemaExecuteJq, functionsExecutionsCreateSwissSystemResponseSchemaExecute, functionsExecutionsCreateSwissSystemResponseSchemaExecuteJq, functionsGetExecute, functionsGetExecuteJq, functionsGetRequestSchemaExecute, functionsGetRequestSchemaExecuteJq, functionsGetResponseSchemaExecute, functionsGetResponseSchemaExecuteJq, functionsInventionsRecursiveCreateAlphaScalarExecute, functionsInventionsRecursiveCreateAlphaScalarExecuteJq, functionsInventionsRecursiveCreateAlphaScalarExecuteStreaming, functionsInventionsRecursiveCreateAlphaScalarExecuteStreamingJq, functionsInventionsRecursiveCreateAlphaScalarRequestSchemaExecute, functionsInventionsRecursiveCreateAlphaScalarRequestSchemaExecuteJq, functionsInventionsRecursiveCreateAlphaScalarResponseSchemaExecute, functionsInventionsRecursiveCreateAlphaScalarResponseSchemaExecuteJq, functionsInventionsRecursiveCreateAlphaVectorExecute, functionsInventionsRecursiveCreateAlphaVectorExecuteJq, functionsInventionsRecursiveCreateAlphaVectorExecuteStreaming, functionsInventionsRecursiveCreateAlphaVectorExecuteStreamingJq, functionsInventionsRecursiveCreateAlphaVectorRequestSchemaExecute, functionsInventionsRecursiveCreateAlphaVectorRequestSchemaExecuteJq, functionsInventionsRecursiveCreateAlphaVectorResponseSchemaExecute, functionsInventionsRecursiveCreateAlphaVectorResponseSchemaExecuteJq, functionsInventionsRecursiveCreateRemoteExecute, functionsInventionsRecursiveCreateRemoteExecuteJq, functionsInventionsRecursiveCreateRemoteExecuteStreaming, functionsInventionsRecursiveCreateRemoteExecuteStreamingJq, functionsInventionsRecursiveCreateRemoteRequestSchemaExecute, functionsInventionsRecursiveCreateRemoteRequestSchemaExecuteJq, functionsInventionsRecursiveCreateRemoteResponseSchemaExecute, functionsInventionsRecursiveCreateRemoteResponseSchemaExecuteJq, functionsInventionsStateGetExecute, functionsInventionsStateGetExecuteJq, functionsInventionsStateGetRequestSchemaExecute, functionsInventionsStateGetRequestSchemaExecuteJq, functionsInventionsStateGetResponseSchemaExecute, functionsInventionsStateGetResponseSchemaExecuteJq, functionsListExecute, functionsListExecuteJq, functionsListRequestSchemaExecute, functionsListRequestSchemaExecuteJq, functionsListResponseSchemaExecute, functionsListResponseSchemaExecuteJq, functionsProfilesGetExecute, functionsProfilesGetExecuteJq, functionsProfilesGetRequestSchemaExecute, functionsProfilesGetRequestSchemaExecuteJq, functionsProfilesGetResponseSchemaExecute, functionsProfilesGetResponseSchemaExecuteJq, functionsProfilesListExecute, functionsProfilesListExecuteJq, functionsProfilesListRequestSchemaExecute, functionsProfilesListRequestSchemaExecuteJq, functionsProfilesListResponseSchemaExecute, functionsProfilesListResponseSchemaExecuteJq, functionsProfilesPublishExecute, functionsProfilesPublishExecuteJq, functionsProfilesPublishRequestSchemaExecute, functionsProfilesPublishRequestSchemaExecuteJq, functionsProfilesPublishResponseSchemaExecute, functionsProfilesPublishResponseSchemaExecuteJq, functionsPublishExecute, functionsPublishExecuteJq, functionsPublishRequestSchemaExecute, functionsPublishRequestSchemaExecuteJq, functionsPublishResponseSchemaExecute, functionsPublishResponseSchemaExecuteJq, invokeCliRequest, listen, logsAgentsCompletionsRequestGetExecute, logsAgentsCompletionsRequestGetExecuteJq, logsAgentsCompletionsRequestGetRequestSchemaExecute, logsAgentsCompletionsRequestGetRequestSchemaExecuteJq, logsAgentsCompletionsRequestGetResponseSchemaExecute, logsAgentsCompletionsRequestGetResponseSchemaExecuteJq, logsAgentsCompletionsRequestMessagesAudioGetExecute, logsAgentsCompletionsRequestMessagesAudioGetExecuteJq, logsAgentsCompletionsRequestMessagesAudioGetRequestSchemaExecute, logsAgentsCompletionsRequestMessagesAudioGetRequestSchemaExecuteJq, logsAgentsCompletionsRequestMessagesAudioGetResponseSchemaExecute, logsAgentsCompletionsRequestMessagesAudioGetResponseSchemaExecuteJq, logsAgentsCompletionsRequestMessagesFileGetExecute, logsAgentsCompletionsRequestMessagesFileGetExecuteJq, logsAgentsCompletionsRequestMessagesFileGetRequestSchemaExecute, logsAgentsCompletionsRequestMessagesFileGetRequestSchemaExecuteJq, logsAgentsCompletionsRequestMessagesFileGetResponseSchemaExecute, logsAgentsCompletionsRequestMessagesFileGetResponseSchemaExecuteJq, logsAgentsCompletionsRequestMessagesImageGetExecute, logsAgentsCompletionsRequestMessagesImageGetExecuteJq, logsAgentsCompletionsRequestMessagesImageGetRequestSchemaExecute, logsAgentsCompletionsRequestMessagesImageGetRequestSchemaExecuteJq, logsAgentsCompletionsRequestMessagesImageGetResponseSchemaExecute, logsAgentsCompletionsRequestMessagesImageGetResponseSchemaExecuteJq, logsAgentsCompletionsRequestMessagesTextGetExecute, logsAgentsCompletionsRequestMessagesTextGetExecuteJq, logsAgentsCompletionsRequestMessagesTextGetRequestSchemaExecute, logsAgentsCompletionsRequestMessagesTextGetRequestSchemaExecuteJq, logsAgentsCompletionsRequestMessagesTextGetResponseSchemaExecute, logsAgentsCompletionsRequestMessagesTextGetResponseSchemaExecuteJq, logsAgentsCompletionsRequestMessagesVideoGetExecute, logsAgentsCompletionsRequestMessagesVideoGetExecuteJq, logsAgentsCompletionsRequestMessagesVideoGetRequestSchemaExecute, logsAgentsCompletionsRequestMessagesVideoGetRequestSchemaExecuteJq, logsAgentsCompletionsRequestMessagesVideoGetResponseSchemaExecute, logsAgentsCompletionsRequestMessagesVideoGetResponseSchemaExecuteJq, logsAgentsCompletionsRequestNotificationsAudioGetExecute, logsAgentsCompletionsRequestNotificationsAudioGetExecuteJq, logsAgentsCompletionsRequestNotificationsAudioGetRequestSchemaExecute, logsAgentsCompletionsRequestNotificationsAudioGetRequestSchemaExecuteJq, logsAgentsCompletionsRequestNotificationsAudioGetResponseSchemaExecute, logsAgentsCompletionsRequestNotificationsAudioGetResponseSchemaExecuteJq, logsAgentsCompletionsRequestNotificationsFileGetExecute, logsAgentsCompletionsRequestNotificationsFileGetExecuteJq, logsAgentsCompletionsRequestNotificationsFileGetRequestSchemaExecute, logsAgentsCompletionsRequestNotificationsFileGetRequestSchemaExecuteJq, logsAgentsCompletionsRequestNotificationsFileGetResponseSchemaExecute, logsAgentsCompletionsRequestNotificationsFileGetResponseSchemaExecuteJq, logsAgentsCompletionsRequestNotificationsImageGetExecute, logsAgentsCompletionsRequestNotificationsImageGetExecuteJq, logsAgentsCompletionsRequestNotificationsImageGetRequestSchemaExecute, logsAgentsCompletionsRequestNotificationsImageGetRequestSchemaExecuteJq, logsAgentsCompletionsRequestNotificationsImageGetResponseSchemaExecute, logsAgentsCompletionsRequestNotificationsImageGetResponseSchemaExecuteJq, logsAgentsCompletionsRequestNotificationsTextGetExecute, logsAgentsCompletionsRequestNotificationsTextGetExecuteJq, logsAgentsCompletionsRequestNotificationsTextGetRequestSchemaExecute, logsAgentsCompletionsRequestNotificationsTextGetRequestSchemaExecuteJq, logsAgentsCompletionsRequestNotificationsTextGetResponseSchemaExecute, logsAgentsCompletionsRequestNotificationsTextGetResponseSchemaExecuteJq, logsAgentsCompletionsRequestNotificationsVideoGetExecute, logsAgentsCompletionsRequestNotificationsVideoGetExecuteJq, logsAgentsCompletionsRequestNotificationsVideoGetRequestSchemaExecute, logsAgentsCompletionsRequestNotificationsVideoGetRequestSchemaExecuteJq, logsAgentsCompletionsRequestNotificationsVideoGetResponseSchemaExecute, logsAgentsCompletionsRequestNotificationsVideoGetResponseSchemaExecuteJq, logsAgentsCompletionsRequestSubscribeExecute, logsAgentsCompletionsRequestSubscribeExecuteJq, logsAgentsCompletionsRequestSubscribeRequestSchemaExecute, logsAgentsCompletionsRequestSubscribeRequestSchemaExecuteJq, logsAgentsCompletionsRequestSubscribeResponseSchemaExecute, logsAgentsCompletionsRequestSubscribeResponseSchemaExecuteJq, logsAgentsCompletionsResponseClearExecute, logsAgentsCompletionsResponseClearExecuteJq, logsAgentsCompletionsResponseClearRequestSchemaExecute, logsAgentsCompletionsResponseClearRequestSchemaExecuteJq, logsAgentsCompletionsResponseClearResponseSchemaExecute, logsAgentsCompletionsResponseClearResponseSchemaExecuteJq, logsAgentsCompletionsResponseContinuationsClearExecute, logsAgentsCompletionsResponseContinuationsClearExecuteJq, logsAgentsCompletionsResponseContinuationsClearRequestSchemaExecute, logsAgentsCompletionsResponseContinuationsClearRequestSchemaExecuteJq, logsAgentsCompletionsResponseContinuationsClearResponseSchemaExecute, logsAgentsCompletionsResponseContinuationsClearResponseSchemaExecuteJq, logsAgentsCompletionsResponseContinuationsGetExecute, logsAgentsCompletionsResponseContinuationsGetExecuteJq, logsAgentsCompletionsResponseContinuationsGetRequestSchemaExecute, logsAgentsCompletionsResponseContinuationsGetRequestSchemaExecuteJq, logsAgentsCompletionsResponseContinuationsGetResponseSchemaExecute, logsAgentsCompletionsResponseContinuationsGetResponseSchemaExecuteJq, logsAgentsCompletionsResponseContinuationsSubscribeExecute, logsAgentsCompletionsResponseContinuationsSubscribeExecuteJq, logsAgentsCompletionsResponseContinuationsSubscribeRequestSchemaExecute, logsAgentsCompletionsResponseContinuationsSubscribeRequestSchemaExecuteJq, logsAgentsCompletionsResponseContinuationsSubscribeResponseSchemaExecute, logsAgentsCompletionsResponseContinuationsSubscribeResponseSchemaExecuteJq, logsAgentsCompletionsResponseGetExecute, logsAgentsCompletionsResponseGetExecuteJq, logsAgentsCompletionsResponseGetRequestSchemaExecute, logsAgentsCompletionsResponseGetRequestSchemaExecuteJq, logsAgentsCompletionsResponseGetResponseSchemaExecute, logsAgentsCompletionsResponseGetResponseSchemaExecuteJq, logsAgentsCompletionsResponseListExecute, logsAgentsCompletionsResponseListExecuteJq, logsAgentsCompletionsResponseListRequestSchemaExecute, logsAgentsCompletionsResponseListRequestSchemaExecuteJq, logsAgentsCompletionsResponseListResponseSchemaExecute, logsAgentsCompletionsResponseListResponseSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantAudioClearExecute, logsAgentsCompletionsResponseMessagesAssistantAudioClearExecuteJq, logsAgentsCompletionsResponseMessagesAssistantAudioClearRequestSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantAudioClearRequestSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantAudioClearResponseSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantAudioClearResponseSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantAudioGetExecute, logsAgentsCompletionsResponseMessagesAssistantAudioGetExecuteJq, logsAgentsCompletionsResponseMessagesAssistantAudioGetRequestSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantAudioGetRequestSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantAudioGetResponseSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantAudioGetResponseSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantAudioSubscribeExecute, logsAgentsCompletionsResponseMessagesAssistantAudioSubscribeExecuteJq, logsAgentsCompletionsResponseMessagesAssistantAudioSubscribeRequestSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantAudioSubscribeRequestSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantAudioSubscribeResponseSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantAudioSubscribeResponseSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantClearExecute, logsAgentsCompletionsResponseMessagesAssistantClearExecuteJq, logsAgentsCompletionsResponseMessagesAssistantClearRequestSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantClearRequestSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantClearResponseSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantClearResponseSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantFileClearExecute, logsAgentsCompletionsResponseMessagesAssistantFileClearExecuteJq, logsAgentsCompletionsResponseMessagesAssistantFileClearRequestSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantFileClearRequestSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantFileClearResponseSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantFileClearResponseSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantFileGetExecute, logsAgentsCompletionsResponseMessagesAssistantFileGetExecuteJq, logsAgentsCompletionsResponseMessagesAssistantFileGetRequestSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantFileGetRequestSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantFileGetResponseSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantFileGetResponseSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantFileSubscribeExecute, logsAgentsCompletionsResponseMessagesAssistantFileSubscribeExecuteJq, logsAgentsCompletionsResponseMessagesAssistantFileSubscribeRequestSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantFileSubscribeRequestSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantFileSubscribeResponseSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantFileSubscribeResponseSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantGetExecute, logsAgentsCompletionsResponseMessagesAssistantGetExecuteJq, logsAgentsCompletionsResponseMessagesAssistantGetRequestSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantGetRequestSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantGetResponseSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantGetResponseSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantImageClearExecute, logsAgentsCompletionsResponseMessagesAssistantImageClearExecuteJq, logsAgentsCompletionsResponseMessagesAssistantImageClearRequestSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantImageClearRequestSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantImageClearResponseSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantImageClearResponseSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantImageGetExecute, logsAgentsCompletionsResponseMessagesAssistantImageGetExecuteJq, logsAgentsCompletionsResponseMessagesAssistantImageGetRequestSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantImageGetRequestSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantImageGetResponseSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantImageGetResponseSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantImageSubscribeExecute, logsAgentsCompletionsResponseMessagesAssistantImageSubscribeExecuteJq, logsAgentsCompletionsResponseMessagesAssistantImageSubscribeRequestSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantImageSubscribeRequestSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantImageSubscribeResponseSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantImageSubscribeResponseSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantLogprobsClearExecute, logsAgentsCompletionsResponseMessagesAssistantLogprobsClearExecuteJq, logsAgentsCompletionsResponseMessagesAssistantLogprobsClearRequestSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantLogprobsClearRequestSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantLogprobsClearResponseSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantLogprobsClearResponseSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantLogprobsGetExecute, logsAgentsCompletionsResponseMessagesAssistantLogprobsGetExecuteJq, logsAgentsCompletionsResponseMessagesAssistantLogprobsGetRequestSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantLogprobsGetRequestSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantLogprobsGetResponseSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantLogprobsGetResponseSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantLogprobsSubscribeExecute, logsAgentsCompletionsResponseMessagesAssistantLogprobsSubscribeExecuteJq, logsAgentsCompletionsResponseMessagesAssistantLogprobsSubscribeRequestSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantLogprobsSubscribeRequestSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantLogprobsSubscribeResponseSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantLogprobsSubscribeResponseSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantReasoningClearExecute, logsAgentsCompletionsResponseMessagesAssistantReasoningClearExecuteJq, logsAgentsCompletionsResponseMessagesAssistantReasoningClearRequestSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantReasoningClearRequestSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantReasoningClearResponseSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantReasoningClearResponseSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantReasoningGetExecute, logsAgentsCompletionsResponseMessagesAssistantReasoningGetExecuteJq, logsAgentsCompletionsResponseMessagesAssistantReasoningGetRequestSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantReasoningGetRequestSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantReasoningGetResponseSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantReasoningGetResponseSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantReasoningSubscribeExecute, logsAgentsCompletionsResponseMessagesAssistantReasoningSubscribeExecuteJq, logsAgentsCompletionsResponseMessagesAssistantReasoningSubscribeRequestSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantReasoningSubscribeRequestSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantReasoningSubscribeResponseSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantReasoningSubscribeResponseSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantRefusalClearExecute, logsAgentsCompletionsResponseMessagesAssistantRefusalClearExecuteJq, logsAgentsCompletionsResponseMessagesAssistantRefusalClearRequestSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantRefusalClearRequestSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantRefusalClearResponseSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantRefusalClearResponseSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantRefusalGetExecute, logsAgentsCompletionsResponseMessagesAssistantRefusalGetExecuteJq, logsAgentsCompletionsResponseMessagesAssistantRefusalGetRequestSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantRefusalGetRequestSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantRefusalGetResponseSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantRefusalGetResponseSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantRefusalSubscribeExecute, logsAgentsCompletionsResponseMessagesAssistantRefusalSubscribeExecuteJq, logsAgentsCompletionsResponseMessagesAssistantRefusalSubscribeRequestSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantRefusalSubscribeRequestSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantRefusalSubscribeResponseSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantRefusalSubscribeResponseSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantSubscribeExecute, logsAgentsCompletionsResponseMessagesAssistantSubscribeExecuteJq, logsAgentsCompletionsResponseMessagesAssistantSubscribeRequestSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantSubscribeRequestSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantSubscribeResponseSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantSubscribeResponseSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantTextGetExecute, logsAgentsCompletionsResponseMessagesAssistantTextGetExecuteJq, logsAgentsCompletionsResponseMessagesAssistantTextGetRequestSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantTextGetRequestSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantTextGetResponseSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantTextGetResponseSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantToolCallsClearExecute, logsAgentsCompletionsResponseMessagesAssistantToolCallsClearExecuteJq, logsAgentsCompletionsResponseMessagesAssistantToolCallsClearRequestSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantToolCallsClearRequestSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantToolCallsClearResponseSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantToolCallsClearResponseSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantToolCallsGetExecute, logsAgentsCompletionsResponseMessagesAssistantToolCallsGetExecuteJq, logsAgentsCompletionsResponseMessagesAssistantToolCallsGetRequestSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantToolCallsGetRequestSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantToolCallsGetResponseSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantToolCallsGetResponseSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantToolCallsSubscribeExecute, logsAgentsCompletionsResponseMessagesAssistantToolCallsSubscribeExecuteJq, logsAgentsCompletionsResponseMessagesAssistantToolCallsSubscribeRequestSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantToolCallsSubscribeRequestSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantToolCallsSubscribeResponseSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantToolCallsSubscribeResponseSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantVideoClearExecute, logsAgentsCompletionsResponseMessagesAssistantVideoClearExecuteJq, logsAgentsCompletionsResponseMessagesAssistantVideoClearRequestSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantVideoClearRequestSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantVideoClearResponseSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantVideoClearResponseSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantVideoGetExecute, logsAgentsCompletionsResponseMessagesAssistantVideoGetExecuteJq, logsAgentsCompletionsResponseMessagesAssistantVideoGetRequestSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantVideoGetRequestSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantVideoGetResponseSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantVideoGetResponseSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantVideoSubscribeExecute, logsAgentsCompletionsResponseMessagesAssistantVideoSubscribeExecuteJq, logsAgentsCompletionsResponseMessagesAssistantVideoSubscribeRequestSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantVideoSubscribeRequestSchemaExecuteJq, logsAgentsCompletionsResponseMessagesAssistantVideoSubscribeResponseSchemaExecute, logsAgentsCompletionsResponseMessagesAssistantVideoSubscribeResponseSchemaExecuteJq, logsAgentsCompletionsResponseMessagesToolAudioGetExecute, logsAgentsCompletionsResponseMessagesToolAudioGetExecuteJq, logsAgentsCompletionsResponseMessagesToolAudioGetRequestSchemaExecute, logsAgentsCompletionsResponseMessagesToolAudioGetRequestSchemaExecuteJq, logsAgentsCompletionsResponseMessagesToolAudioGetResponseSchemaExecute, logsAgentsCompletionsResponseMessagesToolAudioGetResponseSchemaExecuteJq, logsAgentsCompletionsResponseMessagesToolClearExecute, logsAgentsCompletionsResponseMessagesToolClearExecuteJq, logsAgentsCompletionsResponseMessagesToolClearRequestSchemaExecute, logsAgentsCompletionsResponseMessagesToolClearRequestSchemaExecuteJq, logsAgentsCompletionsResponseMessagesToolClearResponseSchemaExecute, logsAgentsCompletionsResponseMessagesToolClearResponseSchemaExecuteJq, logsAgentsCompletionsResponseMessagesToolFileGetExecute, logsAgentsCompletionsResponseMessagesToolFileGetExecuteJq, logsAgentsCompletionsResponseMessagesToolFileGetRequestSchemaExecute, logsAgentsCompletionsResponseMessagesToolFileGetRequestSchemaExecuteJq, logsAgentsCompletionsResponseMessagesToolFileGetResponseSchemaExecute, logsAgentsCompletionsResponseMessagesToolFileGetResponseSchemaExecuteJq, logsAgentsCompletionsResponseMessagesToolGetExecute, logsAgentsCompletionsResponseMessagesToolGetExecuteJq, logsAgentsCompletionsResponseMessagesToolGetRequestSchemaExecute, logsAgentsCompletionsResponseMessagesToolGetRequestSchemaExecuteJq, logsAgentsCompletionsResponseMessagesToolGetResponseSchemaExecute, logsAgentsCompletionsResponseMessagesToolGetResponseSchemaExecuteJq, logsAgentsCompletionsResponseMessagesToolImageGetExecute, logsAgentsCompletionsResponseMessagesToolImageGetExecuteJq, logsAgentsCompletionsResponseMessagesToolImageGetRequestSchemaExecute, logsAgentsCompletionsResponseMessagesToolImageGetRequestSchemaExecuteJq, logsAgentsCompletionsResponseMessagesToolImageGetResponseSchemaExecute, logsAgentsCompletionsResponseMessagesToolImageGetResponseSchemaExecuteJq, logsAgentsCompletionsResponseMessagesToolSubscribeExecute, logsAgentsCompletionsResponseMessagesToolSubscribeExecuteJq, logsAgentsCompletionsResponseMessagesToolSubscribeRequestSchemaExecute, logsAgentsCompletionsResponseMessagesToolSubscribeRequestSchemaExecuteJq, logsAgentsCompletionsResponseMessagesToolSubscribeResponseSchemaExecute, logsAgentsCompletionsResponseMessagesToolSubscribeResponseSchemaExecuteJq, logsAgentsCompletionsResponseMessagesToolTextGetExecute, logsAgentsCompletionsResponseMessagesToolTextGetExecuteJq, logsAgentsCompletionsResponseMessagesToolTextGetRequestSchemaExecute, logsAgentsCompletionsResponseMessagesToolTextGetRequestSchemaExecuteJq, logsAgentsCompletionsResponseMessagesToolTextGetResponseSchemaExecute, logsAgentsCompletionsResponseMessagesToolTextGetResponseSchemaExecuteJq, logsAgentsCompletionsResponseMessagesToolVideoGetExecute, logsAgentsCompletionsResponseMessagesToolVideoGetExecuteJq, logsAgentsCompletionsResponseMessagesToolVideoGetRequestSchemaExecute, logsAgentsCompletionsResponseMessagesToolVideoGetRequestSchemaExecuteJq, logsAgentsCompletionsResponseMessagesToolVideoGetResponseSchemaExecute, logsAgentsCompletionsResponseMessagesToolVideoGetResponseSchemaExecuteJq, logsAgentsCompletionsResponseSubscribeExecute, logsAgentsCompletionsResponseSubscribeExecuteJq, logsAgentsCompletionsResponseSubscribeRequestSchemaExecute, logsAgentsCompletionsResponseSubscribeRequestSchemaExecuteJq, logsAgentsCompletionsResponseSubscribeResponseSchemaExecute, logsAgentsCompletionsResponseSubscribeResponseSchemaExecuteJq, logsClearExecute, logsClearExecuteJq, logsClearRequestSchemaExecute, logsClearRequestSchemaExecuteJq, logsClearResponseSchemaExecute, logsClearResponseSchemaExecuteJq, logsFunctionsExecutionsRequestGetExecute, logsFunctionsExecutionsRequestGetExecuteJq, logsFunctionsExecutionsRequestGetRequestSchemaExecute, logsFunctionsExecutionsRequestGetRequestSchemaExecuteJq, logsFunctionsExecutionsRequestGetResponseSchemaExecute, logsFunctionsExecutionsRequestGetResponseSchemaExecuteJq, logsFunctionsExecutionsRequestSubscribeExecute, logsFunctionsExecutionsRequestSubscribeExecuteJq, logsFunctionsExecutionsRequestSubscribeRequestSchemaExecute, logsFunctionsExecutionsRequestSubscribeRequestSchemaExecuteJq, logsFunctionsExecutionsRequestSubscribeResponseSchemaExecute, logsFunctionsExecutionsRequestSubscribeResponseSchemaExecuteJq, logsFunctionsExecutionsResponseClearExecute, logsFunctionsExecutionsResponseClearExecuteJq, logsFunctionsExecutionsResponseClearRequestSchemaExecute, logsFunctionsExecutionsResponseClearRequestSchemaExecuteJq, logsFunctionsExecutionsResponseClearResponseSchemaExecute, logsFunctionsExecutionsResponseClearResponseSchemaExecuteJq, logsFunctionsExecutionsResponseGetExecute, logsFunctionsExecutionsResponseGetExecuteJq, logsFunctionsExecutionsResponseGetRequestSchemaExecute, logsFunctionsExecutionsResponseGetRequestSchemaExecuteJq, logsFunctionsExecutionsResponseGetResponseSchemaExecute, logsFunctionsExecutionsResponseGetResponseSchemaExecuteJq, logsFunctionsExecutionsResponseListExecute, logsFunctionsExecutionsResponseListExecuteJq, logsFunctionsExecutionsResponseListRequestSchemaExecute, logsFunctionsExecutionsResponseListRequestSchemaExecuteJq, logsFunctionsExecutionsResponseListResponseSchemaExecute, logsFunctionsExecutionsResponseListResponseSchemaExecuteJq, logsFunctionsExecutionsResponseRetryTokensClearExecute, logsFunctionsExecutionsResponseRetryTokensClearExecuteJq, logsFunctionsExecutionsResponseRetryTokensClearRequestSchemaExecute, logsFunctionsExecutionsResponseRetryTokensClearRequestSchemaExecuteJq, logsFunctionsExecutionsResponseRetryTokensClearResponseSchemaExecute, logsFunctionsExecutionsResponseRetryTokensClearResponseSchemaExecuteJq, logsFunctionsExecutionsResponseRetryTokensGetExecute, logsFunctionsExecutionsResponseRetryTokensGetExecuteJq, logsFunctionsExecutionsResponseRetryTokensGetRequestSchemaExecute, logsFunctionsExecutionsResponseRetryTokensGetRequestSchemaExecuteJq, logsFunctionsExecutionsResponseRetryTokensGetResponseSchemaExecute, logsFunctionsExecutionsResponseRetryTokensGetResponseSchemaExecuteJq, logsFunctionsExecutionsResponseRetryTokensSubscribeExecute, logsFunctionsExecutionsResponseRetryTokensSubscribeExecuteJq, logsFunctionsExecutionsResponseRetryTokensSubscribeRequestSchemaExecute, logsFunctionsExecutionsResponseRetryTokensSubscribeRequestSchemaExecuteJq, logsFunctionsExecutionsResponseRetryTokensSubscribeResponseSchemaExecute, logsFunctionsExecutionsResponseRetryTokensSubscribeResponseSchemaExecuteJq, logsFunctionsExecutionsResponseSubscribeExecute, logsFunctionsExecutionsResponseSubscribeExecuteJq, logsFunctionsExecutionsResponseSubscribeRequestSchemaExecute, logsFunctionsExecutionsResponseSubscribeRequestSchemaExecuteJq, logsFunctionsExecutionsResponseSubscribeResponseSchemaExecute, logsFunctionsExecutionsResponseSubscribeResponseSchemaExecuteJq, logsFunctionsInventionsRecursiveRequestGetExecute, logsFunctionsInventionsRecursiveRequestGetExecuteJq, logsFunctionsInventionsRecursiveRequestGetRequestSchemaExecute, logsFunctionsInventionsRecursiveRequestGetRequestSchemaExecuteJq, logsFunctionsInventionsRecursiveRequestGetResponseSchemaExecute, logsFunctionsInventionsRecursiveRequestGetResponseSchemaExecuteJq, logsFunctionsInventionsRecursiveRequestSubscribeExecute, logsFunctionsInventionsRecursiveRequestSubscribeExecuteJq, logsFunctionsInventionsRecursiveRequestSubscribeRequestSchemaExecute, logsFunctionsInventionsRecursiveRequestSubscribeRequestSchemaExecuteJq, logsFunctionsInventionsRecursiveRequestSubscribeResponseSchemaExecute, logsFunctionsInventionsRecursiveRequestSubscribeResponseSchemaExecuteJq, logsFunctionsInventionsRecursiveResponseClearExecute, logsFunctionsInventionsRecursiveResponseClearExecuteJq, logsFunctionsInventionsRecursiveResponseClearRequestSchemaExecute, logsFunctionsInventionsRecursiveResponseClearRequestSchemaExecuteJq, logsFunctionsInventionsRecursiveResponseClearResponseSchemaExecute, logsFunctionsInventionsRecursiveResponseClearResponseSchemaExecuteJq, logsFunctionsInventionsRecursiveResponseGetExecute, logsFunctionsInventionsRecursiveResponseGetExecuteJq, logsFunctionsInventionsRecursiveResponseGetRequestSchemaExecute, logsFunctionsInventionsRecursiveResponseGetRequestSchemaExecuteJq, logsFunctionsInventionsRecursiveResponseGetResponseSchemaExecute, logsFunctionsInventionsRecursiveResponseGetResponseSchemaExecuteJq, logsFunctionsInventionsRecursiveResponseListExecute, logsFunctionsInventionsRecursiveResponseListExecuteJq, logsFunctionsInventionsRecursiveResponseListRequestSchemaExecute, logsFunctionsInventionsRecursiveResponseListRequestSchemaExecuteJq, logsFunctionsInventionsRecursiveResponseListResponseSchemaExecute, logsFunctionsInventionsRecursiveResponseListResponseSchemaExecuteJq, logsFunctionsInventionsRecursiveResponseSubscribeExecute, logsFunctionsInventionsRecursiveResponseSubscribeExecuteJq, logsFunctionsInventionsRecursiveResponseSubscribeRequestSchemaExecute, logsFunctionsInventionsRecursiveResponseSubscribeRequestSchemaExecuteJq, logsFunctionsInventionsRecursiveResponseSubscribeResponseSchemaExecute, logsFunctionsInventionsRecursiveResponseSubscribeResponseSchemaExecuteJq, logsFunctionsInventionsRequestGetExecute, logsFunctionsInventionsRequestGetExecuteJq, logsFunctionsInventionsRequestGetRequestSchemaExecute, logsFunctionsInventionsRequestGetRequestSchemaExecuteJq, logsFunctionsInventionsRequestGetResponseSchemaExecute, logsFunctionsInventionsRequestGetResponseSchemaExecuteJq, logsFunctionsInventionsRequestSubscribeExecute, logsFunctionsInventionsRequestSubscribeExecuteJq, logsFunctionsInventionsRequestSubscribeRequestSchemaExecute, logsFunctionsInventionsRequestSubscribeRequestSchemaExecuteJq, logsFunctionsInventionsRequestSubscribeResponseSchemaExecute, logsFunctionsInventionsRequestSubscribeResponseSchemaExecuteJq, logsFunctionsInventionsResponseClearExecute, logsFunctionsInventionsResponseClearExecuteJq, logsFunctionsInventionsResponseClearRequestSchemaExecute, logsFunctionsInventionsResponseClearRequestSchemaExecuteJq, logsFunctionsInventionsResponseClearResponseSchemaExecute, logsFunctionsInventionsResponseClearResponseSchemaExecuteJq, logsFunctionsInventionsResponseGetExecute, logsFunctionsInventionsResponseGetExecuteJq, logsFunctionsInventionsResponseGetRequestSchemaExecute, logsFunctionsInventionsResponseGetRequestSchemaExecuteJq, logsFunctionsInventionsResponseGetResponseSchemaExecute, logsFunctionsInventionsResponseGetResponseSchemaExecuteJq, logsFunctionsInventionsResponseListExecute, logsFunctionsInventionsResponseListExecuteJq, logsFunctionsInventionsResponseListRequestSchemaExecute, logsFunctionsInventionsResponseListRequestSchemaExecuteJq, logsFunctionsInventionsResponseListResponseSchemaExecute, logsFunctionsInventionsResponseListResponseSchemaExecuteJq, logsFunctionsInventionsResponseSubscribeExecute, logsFunctionsInventionsResponseSubscribeExecuteJq, logsFunctionsInventionsResponseSubscribeRequestSchemaExecute, logsFunctionsInventionsResponseSubscribeRequestSchemaExecuteJq, logsFunctionsInventionsResponseSubscribeResponseSchemaExecute, logsFunctionsInventionsResponseSubscribeResponseSchemaExecuteJq, logsVectorCompletionsRequestGetExecute, logsVectorCompletionsRequestGetExecuteJq, logsVectorCompletionsRequestGetRequestSchemaExecute, logsVectorCompletionsRequestGetRequestSchemaExecuteJq, logsVectorCompletionsRequestGetResponseSchemaExecute, logsVectorCompletionsRequestGetResponseSchemaExecuteJq, logsVectorCompletionsRequestSubscribeExecute, logsVectorCompletionsRequestSubscribeExecuteJq, logsVectorCompletionsRequestSubscribeRequestSchemaExecute, logsVectorCompletionsRequestSubscribeRequestSchemaExecuteJq, logsVectorCompletionsRequestSubscribeResponseSchemaExecute, logsVectorCompletionsRequestSubscribeResponseSchemaExecuteJq, logsVectorCompletionsResponseClearExecute, logsVectorCompletionsResponseClearExecuteJq, logsVectorCompletionsResponseClearRequestSchemaExecute, logsVectorCompletionsResponseClearRequestSchemaExecuteJq, logsVectorCompletionsResponseClearResponseSchemaExecute, logsVectorCompletionsResponseClearResponseSchemaExecuteJq, logsVectorCompletionsResponseGetExecute, logsVectorCompletionsResponseGetExecuteJq, logsVectorCompletionsResponseGetRequestSchemaExecute, logsVectorCompletionsResponseGetRequestSchemaExecuteJq, logsVectorCompletionsResponseGetResponseSchemaExecute, logsVectorCompletionsResponseGetResponseSchemaExecuteJq, logsVectorCompletionsResponseListExecute, logsVectorCompletionsResponseListExecuteJq, logsVectorCompletionsResponseListRequestSchemaExecute, logsVectorCompletionsResponseListRequestSchemaExecuteJq, logsVectorCompletionsResponseListResponseSchemaExecute, logsVectorCompletionsResponseListResponseSchemaExecuteJq, logsVectorCompletionsResponseSubscribeExecute, logsVectorCompletionsResponseSubscribeExecuteJq, logsVectorCompletionsResponseSubscribeRequestSchemaExecute, logsVectorCompletionsResponseSubscribeRequestSchemaExecuteJq, logsVectorCompletionsResponseSubscribeResponseSchemaExecute, logsVectorCompletionsResponseSubscribeResponseSchemaExecuteJq, mcpKillExecute, mcpKillExecuteJq, mcpKillRequestSchemaExecute, mcpKillRequestSchemaExecuteJq, mcpKillResponseSchemaExecute, mcpKillResponseSchemaExecuteJq, mcpSpawnExecute, mcpSpawnExecuteJq, mcpSpawnRequestSchemaExecute, mcpSpawnRequestSchemaExecuteJq, mcpSpawnResponseSchemaExecute, mcpSpawnResponseSchemaExecuteJq, pluginsGetExecute, pluginsGetExecuteJq, pluginsGetRequestSchemaExecute, pluginsGetRequestSchemaExecuteJq, pluginsGetResponseSchemaExecute, pluginsGetResponseSchemaExecuteJq, pluginsInstallFilesystemExecute, pluginsInstallFilesystemExecuteJq, pluginsInstallFilesystemRequestSchemaExecute, pluginsInstallFilesystemRequestSchemaExecuteJq, pluginsInstallFilesystemResponseSchemaExecute, pluginsInstallFilesystemResponseSchemaExecuteJq, pluginsInstallGithubExecute, pluginsInstallGithubExecuteJq, pluginsInstallGithubRequestSchemaExecute, pluginsInstallGithubRequestSchemaExecuteJq, pluginsInstallGithubResponseSchemaExecute, pluginsInstallGithubResponseSchemaExecuteJq, pluginsListExecute, pluginsListExecuteJq, pluginsListRequestSchemaExecute, pluginsListRequestSchemaExecuteJq, pluginsListResponseSchemaExecute, pluginsListResponseSchemaExecuteJq, pluginsRunExecute, pluginsRunExecuteJq, pluginsRunRequestSchemaExecute, pluginsRunRequestSchemaExecuteJq, pluginsRunResponseSchemaExecute, pluginsRunResponseSchemaExecuteJq, swarmsGetExecute, swarmsGetExecuteJq, swarmsGetRequestSchemaExecute, swarmsGetRequestSchemaExecuteJq, swarmsGetResponseSchemaExecute, swarmsGetResponseSchemaExecuteJq, swarmsListExecute, swarmsListExecuteJq, swarmsListRequestSchemaExecute, swarmsListRequestSchemaExecuteJq, swarmsListResponseSchemaExecute, swarmsListResponseSchemaExecuteJq, swarmsPublishExecute, swarmsPublishExecuteJq, swarmsPublishRequestSchemaExecute, swarmsPublishRequestSchemaExecuteJq, swarmsPublishResponseSchemaExecute, swarmsPublishResponseSchemaExecuteJq, toolsGetExecute, toolsGetExecuteJq, toolsGetRequestSchemaExecute, toolsGetRequestSchemaExecuteJq, toolsGetResponseSchemaExecute, toolsGetResponseSchemaExecuteJq, toolsInstallExecute, toolsInstallExecuteJq, toolsInstallRequestSchemaExecute, toolsInstallRequestSchemaExecuteJq, toolsInstallResponseSchemaExecute, toolsInstallResponseSchemaExecuteJq, toolsListExecute, toolsListExecuteJq, toolsListRequestSchemaExecute, toolsListRequestSchemaExecuteJq, toolsListResponseSchemaExecute, toolsListResponseSchemaExecuteJq, toolsRunExecute, toolsRunExecuteJq, toolsRunRequestSchemaExecute, toolsRunRequestSchemaExecuteJq, toolsRunResponseSchemaExecute, toolsRunResponseSchemaExecuteJq, updateExecute, updateExecuteJq, updateRequestSchemaExecute, updateRequestSchemaExecuteJq, updateResponseSchemaExecute, updateResponseSchemaExecuteJq, viewerGenerateSecretSignaturePairExecute, viewerGenerateSecretSignaturePairExecuteJq, viewerGenerateSecretSignaturePairRequestSchemaExecute, viewerGenerateSecretSignaturePairRequestSchemaExecuteJq, viewerGenerateSecretSignaturePairResponseSchemaExecute, viewerGenerateSecretSignaturePairResponseSchemaExecuteJq, viewerKillExecute, viewerKillExecuteJq, viewerKillRequestSchemaExecute, viewerKillRequestSchemaExecuteJq, viewerKillResponseSchemaExecute, viewerKillResponseSchemaExecuteJq, viewerSendExecute, viewerSendExecuteJq, viewerSendRequestSchemaExecute, viewerSendRequestSchemaExecuteJq, viewerSendResponseSchemaExecute, viewerSendResponseSchemaExecuteJq, viewerSpawnExecute, viewerSpawnExecuteJq, viewerSpawnRequestSchemaExecute, viewerSpawnRequestSchemaExecuteJq, viewerSpawnResponseSchemaExecute, viewerSpawnResponseSchemaExecuteJq };
