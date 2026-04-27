# MCP Endpoints

Every JSON-RPC method an MCP **server** must understand (or, for server-originated messages, must be prepared to send) to be fully spec-compliant against MCP `2025-06-18`.

Each row lists:

- **Method** — the JSON-RPC `method` string sent on the wire.
- **Direction** — who originates the message.
- **Kind** — Request (expects a response) or Notification (no response).
- **Server obligation** — what a server must do to comply.
- **Source** — where the requirement is documented (spec page or in-tree client file).

Spec base: <https://modelcontextprotocol.io/specification/2025-06-18>.
Authoritative schema: <https://github.com/modelcontextprotocol/specification/blob/main/schema/2025-06-18/schema.ts>.

---

## Lifecycle (mandatory for every server)

| Method | Direction | Kind | Server obligation | Source |
|---|---|---|---|---|
| `initialize` | client → server | Request | Negotiate protocol version + capabilities; return `InitializeResult { protocolVersion, capabilities, serverInfo, instructions?, _meta? }`. | Spec: `/specification/2025-06-18/basic/lifecycle#initialization`. In-tree usage: `objectiveai-rs/src/mcp/client.rs:87-99` (the `json!({...})` request body) and `initialize_result.rs` (typed response). |
| `notifications/initialized` | client → server | Notification | Accept and gate the rest of normal operation behind it; do not send server-initiated requests until received. | Spec: `/specification/2025-06-18/basic/lifecycle#initialization`. In-tree usage: `objectiveai-rs/src/mcp/client.rs:173` (sent by the client right after `initialize` succeeds). |
| `ping` | bidirectional | Request | Respond with an empty result; **also** be able to originate `ping` to detect dead clients. | Spec: `/specification/2025-06-18/basic/utilities/ping`. Schema: `schema.ts` `PingRequest`. |

## Cross-cutting utilities (mandatory whenever the corresponding feature is in flight)

| Method | Direction | Kind | Server obligation | Source |
|---|---|---|---|---|
| `notifications/cancelled` | bidirectional | Notification | When received, abort the in-flight request named by `requestId` and stop sending its responses; when originating, may send to abort outbound requests still in flight. | Spec: `/specification/2025-06-18/basic/utilities/cancellation`. Schema: `CancelledNotification`. |
| `notifications/progress` | bidirectional | Notification | When the request carried a `progressToken`, may emit progress updates; must accept incoming progress for any request our server originates that carries one. | Spec: `/specification/2025-06-18/basic/utilities/progress`. Schema: `ProgressNotification`. |

## Tools (advertised by `capabilities.tools`)

| Method | Direction | Kind | Server obligation | Source |
|---|---|---|---|---|
| `tools/list` | client → server | Request | Return `{ tools: Tool[], nextCursor? }`; honor opaque pagination cursor. | Spec: `/specification/2025-06-18/server/tools`. In-tree usage: `objectiveai-rs/src/mcp/connection.rs:368-379` (`rpc_list_tools`), `tool/list.rs` (typed `ListToolsRequest` / `ListToolsResult`). |
| `tools/call` | client → server | Request | Invoke the named tool with `arguments`; return `CallToolResult { content[], structuredContent?, isError?, _meta? }`. | Spec: `/specification/2025-06-18/server/tools`. In-tree usage: `objectiveai-rs/src/mcp/connection.rs:392-409` (`call_tool`), `tool/call.rs` (typed params + result). |
| `notifications/tools/list_changed` | server → client | Notification | If `capabilities.tools.listChanged == true`, emit whenever the tool inventory mutates so clients can re-list. | Spec: `/specification/2025-06-18/server/tools`. In-tree usage: `objectiveai-rs/src/mcp/connection.rs:712-715` (the listener that triggers a refresh). |

## Resources (advertised by `capabilities.resources`)

| Method | Direction | Kind | Server obligation | Source |
|---|---|---|---|---|
| `resources/list` | client → server | Request | Return `{ resources: Resource[], nextCursor? }`; paginated. | Spec: `/specification/2025-06-18/server/resources`. In-tree usage: `objectiveai-rs/src/mcp/connection.rs:575-586` (`rpc_list_resources`), `resource/list.rs`. |
| `resources/templates/list` | client → server | Request | Return `{ resourceTemplates: ResourceTemplate[], nextCursor? }`; paginated. Required even if you have no templates (return an empty array). | Spec: `/specification/2025-06-18/server/resources` (URI Templates section). Schema: `ListResourceTemplatesRequest`. |
| `resources/read` | client → server | Request | Return `{ contents: ResourceContents[] }` for the requested URI (text or blob form). | Spec: `/specification/2025-06-18/server/resources`. In-tree usage: `objectiveai-rs/src/mcp/connection.rs:599-610` (`read_resource`), `resource/read.rs`. |
| `resources/subscribe` | client → server | Request | If `capabilities.resources.subscribe == true`, register the client for `resources/updated` events on the given URI. | Spec: `/specification/2025-06-18/server/resources` (Subscriptions section). Schema: `SubscribeRequest`. |
| `resources/unsubscribe` | client → server | Request | Companion to `subscribe`; remove the registration. Also gated by `capabilities.resources.subscribe`. | Spec: `/specification/2025-06-18/server/resources` (Subscriptions section). Schema: `UnsubscribeRequest`. |
| `notifications/resources/list_changed` | server → client | Notification | If `capabilities.resources.listChanged == true`, emit on inventory change. | Spec: `/specification/2025-06-18/server/resources`. In-tree usage: `objectiveai-rs/src/mcp/connection.rs:716-718`. |
| `notifications/resources/updated` | server → client | Notification | For each subscribed URI, emit when the underlying content changes. | Spec: `/specification/2025-06-18/server/resources` (Subscriptions section). Schema: `ResourceUpdatedNotification`. |

## Prompts (advertised by `capabilities.prompts`)

| Method | Direction | Kind | Server obligation | Source |
|---|---|---|---|---|
| `prompts/list` | client → server | Request | Return `{ prompts: Prompt[], nextCursor? }`; paginated. | Spec: `/specification/2025-06-18/server/prompts`. Schema: `ListPromptsRequest`. |
| `prompts/get` | client → server | Request | Render the named prompt with provided `arguments`; return `{ description?, messages: PromptMessage[] }`. | Spec: `/specification/2025-06-18/server/prompts`. Schema: `GetPromptRequest`. |
| `notifications/prompts/list_changed` | server → client | Notification | If `capabilities.prompts.listChanged == true`, emit on inventory change. | Spec: `/specification/2025-06-18/server/prompts`. Schema: `PromptListChangedNotification`. |

## Logging (advertised by `capabilities.logging`)

| Method | Direction | Kind | Server obligation | Source |
|---|---|---|---|---|
| `logging/setLevel` | client → server | Request | Apply the requested verbosity level (`debug`/`info`/`notice`/`warning`/`error`/`critical`/`alert`/`emergency`); subsequent log emissions must respect it. | Spec: `/specification/2025-06-18/server/utilities/logging`. Schema: `SetLevelRequest`. |
| `notifications/message` | server → client | Notification | Emit log messages above the configured threshold with `{ level, logger?, data }`. | Spec: `/specification/2025-06-18/server/utilities/logging`. Schema: `LoggingMessageNotification`. |

## Completion (advertised by `capabilities.completions`)

| Method | Direction | Kind | Server obligation | Source |
|---|---|---|---|---|
| `completion/complete` | client → server | Request | Given a `ref/prompt` or `ref/resource` and the in-progress `argument` (plus optional already-resolved `context.arguments`), return up to 100 `completion.values` plus optional `total` and `hasMore`. | Spec: `/specification/2025-06-18/server/utilities/completion` (verbatim section "Requesting Completions"). |

## Sampling — server-originated (gated by client `capabilities.sampling`)

The server may *send* this if the client advertised `sampling`. Required on the server side only if you intend to use it; if you do, you must be ready to handle the response (including the `decline` / `cancel` shapes) and not block forever.

| Method | Direction | Kind | Server obligation | Source |
|---|---|---|---|---|
| `sampling/createMessage` | server → client | Request | Construct `messages`, `modelPreferences`, `systemPrompt?`, `maxTokens`, etc.; handle the response (`role`, `content`, `model`, `stopReason`) or a JSON-RPC error (e.g. `-1` "User rejected"). | Spec: `/specification/2025-06-18/client/sampling` (verbatim section "Creating Messages"). |

## Roots — server-originated (gated by client `capabilities.roots`)

| Method | Direction | Kind | Server obligation | Source |
|---|---|---|---|---|
| `roots/list` | server → client | Request | Optional. If used, send the request and consume `{ roots: Root[] }`; tolerate `-32601 "Roots not supported"`. | Spec: `/specification/2025-06-18/client/roots` (verbatim section "Listing Roots"). |
| `notifications/roots/list_changed` | client → server | Notification | Must accept (and re-fetch via `roots/list` if the server cares about root membership). | Spec: `/specification/2025-06-18/client/roots` (verbatim section "Root List Changes"). |

## Elicitation — server-originated (gated by client `capabilities.elicitation`)

| Method | Direction | Kind | Server obligation | Source |
|---|---|---|---|---|
| `elicitation/create` | server → client | Request | Optional. If used, send `{ message, requestedSchema }` (flat-object subset of JSON Schema); handle three response actions: `accept` (with `content`), `decline`, `cancel`. **MUST NOT** be used to request sensitive info. | Spec: `/specification/2025-06-18/client/elicitation` (verbatim section "Creating Elicitation Requests"). |

---

## What is *not* on this list (and why)

- **Transport-level concerns** — `Mcp-Session-Id` headers, SSE framing, the GET-stream for server-initiated messages, OAuth `WWW-Authenticate` flows. These are required by `/specification/2025-06-18/basic/transports` (Streamable HTTP) but they are not JSON-RPC methods. Already partially handled in `objectiveai-rs/src/mcp/client.rs:101-135` (initialize POST + session-id extraction) and `connection.rs:676-726` (GET SSE listener).
- **JSON-RPC envelope mechanics** — the `jsonrpc: "2.0"` framing, batched requests, error code values (`-32700`/`-32600`/`-32601`/`-32602`/`-32603`). These are JSON-RPC 2.0, not MCP.
- **`completions` capability marker named "`completions`"** vs the method `completion/complete` — yes, the capability key and the method root differ. Not a typo. Source: `initialize_result.rs:111` (`CompletionsCapability`) and the spec page above.

## Capability matrix — which methods can be skipped

| Server capability | If absent, the server may omit |
|---|---|
| `tools` | `tools/list`, `tools/call`, `notifications/tools/list_changed` |
| `resources` | `resources/list`, `resources/templates/list`, `resources/read`, `notifications/resources/list_changed` |
| `resources.subscribe == true` | `resources/subscribe`, `resources/unsubscribe`, `notifications/resources/updated` |
| `prompts` | `prompts/list`, `prompts/get`, `notifications/prompts/list_changed` |
| `logging` | `logging/setLevel`, `notifications/message` |
| `completions` | `completion/complete` |

The lifecycle methods (`initialize`, `notifications/initialized`, `ping`) and the cross-cutting notifications (`notifications/cancelled`, `notifications/progress`) are **not** capability-gated and must be supported unconditionally.
