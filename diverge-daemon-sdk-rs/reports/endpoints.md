# The ObjectiveAI daemon's endpoints — every one, as the crate serves them

From `objectiveai-daemon/src/` and the request types it dispatches on
in `objectiveai-sdk-rs/src/cli/command/`, read on 2026-09-17 at
commit `478fa3749`. This is the surface the Diverge daemon's SDK
starts from; not all of it will be kept. Nothing here is from memory
of the product.

One axum router serves everything on the daemon's configured
`address:port` (`http/daemon_stream.rs::serve_http`). Every HTTP
route authenticates with the `X-OBJECTIVEAI-SIGNATURE` header,
`sha256=<hex(SHA256(secret))>`, compared in constant time, `401` on a
mismatch and ignored when no secret is configured
(`http/daemon_auth.rs`). CORS is fully permissive. There are no local
sockets in the client surface any more: what were `daemon.sock`,
`agents.sock`, `conversation.sock` and `laboratories.sock` are
in-process hubs on the global context.

## The routes

`http/daemon_stream.rs` lines 141–231.

| Route | Request | Response | Purpose |
|---|---|---|---|
| `/mcp` (nested service: `POST`, `GET` SSE, `DELETE`) | JSON-RPC MCP messages; `Mcp-Session-Id`; `X-OBJECTIVEAI-*` identity headers on initialize | JSON-RPC, SSE | The daemon's own MCP server, behind the signature middleware |
| `GET /listen` | headers only | SSE of raw broadcast frames | The activity tee of every CLI run in the daemon |
| `POST /execute` | body: one `cli::command::Request` as JSON, plus `X-OBJECTIVEAI-*` identity headers | SSE, one `data:` per item; body close is end of stream | Run one CLI command in-process and stream its items back |
| `GET /agents/instances/list` | none | SSE of `AgentEvent` | Live feed of every agent instance: `Snapshot`, then `Activated` and `Deactivated` |
| `GET /agents/instances/{*aih}` | the agent instance hierarchy as a wildcard path (hierarchies contain `/`) | SSE of `AgentInstanceEvent` | One agent's whole conversation: the status record, a replay from the database, `Live`, then live rows |
| `ANY /laboratory` | WebSocket upgrade; frame 1 `HostIdentify`, frame 2 `AuthEnvelope` | WebSocket, duplex | The one WebSocket: laboratory hosts dial in and serve their containers |
| `GET /laboratories/list` | none | SSE of `LaboratoryEvent` | Live laboratories on connected hosts: `Snapshot`, then `Upserted` and `Removed` |
| `GET /laboratories/{id}` | path `id`; optional `?machine=&machine_state=` | SSE of `LaboratoryInstanceEvent` | One laboratory's record and attachments, the whole value re-sent on change |
| `GET /laboratories/{id}/filetree` | path `id`; optional `?machine=&machine_state=` | SSE of `FileTreeEvent`; `404` when no host serves it, `502` when the container will not start or the host is lost | The laboratory's live file tree, held open until a real snapshot exists |
| `GET /channels` | none | SSE of `ChannelEvent` | The duplex-channel offer lifecycle |
| `POST /channels/{id}/accept` | path `id`, no body | `200` with `{ secret }`; `404` unknown or withdrawn; `409` already accepted | First-wins accept of a pending channel offer, returning the owner secret |
| `GET /plugins/{owner}/{name}/{version}/viewer` | three path segments, `version` a `v`-prefixed tag | a `tar.gz` body streamed, `X-OBJECTIVEAI-SHA` header; `400`, `404`, `503`, `500` | Build a plugin's viewer extension on a laboratory host and stream the artifact |

## `GET /listen` — the frames

`http/daemon_stream.rs` (`listen_handler`, `listen_stream`), `run.rs`
lines 317–500, `objectiveai-sdk-rs/src/daemon/command_listener/wire.rs`.

| Frame | Shape | Meaning |
|---|---|---|
| first of a run | `ListenerRequest` — `{ agent_instance_hierarchy, agent_id, agent_full_id, agent_remote, response_id, response_ids, plugin_owner, plugin_name, plugin_version, task, id, value: Request }` | A new run id and the exact request that started it, stamped with the producer's identity |
| then, repeated | `ListenerResponse` — `{ id, value }` | One response item of that run, before any jq or python transform |
| last | `ListenerEnd` — `{ id, end: true }` | That run's stream is complete |

The `id` is the only discriminant: `end: true` is the terminator, a
known id a response, a new id a request. Frames are raw JSON values.
Every run is teed except `daemon/spawn` with `foreground: true`. A
subscriber that lags drops frames and continues. Runs driven through
`/mcp` are not teed.

## `POST /execute` — the request kinds

The body is the untagged aggregate `cli::command::Request`
(`objectiveai-sdk-rs/src/cli/command/command.rs`), discriminated by
the mandatory `path_type` string on each leaf, for example
`"agents/message"`. Every leaf also flattens
`RequestBase { jq, python, timeout_seconds, max_tokens }`. A leaf with
a `Response` emits exactly one item; a leaf with a `ResponseItem`
streams many. `Ok` is the wire string `"Ok"`.

There are 348 dispatchable kinds: the 116 command leaves below, plus
for each leaf `P` the two introspection kinds `P/request_schema` and
`P/response_schema`, each taking only `path_type` and the base and
answering the JSON Schema of that leaf's request or response type.

### agents

`objectiveai-sdk-rs/src/cli/command/agents/`, handlers
`objectiveai-daemon/src/command/agents/`.

| Kind | Request | Response | Purpose |
|---|---|---|---|
| `agents/enqueue` | `agent: AgentSelector`, `message: RequestMessage`, `key: Option<String>` | `{ id, agent_instance_hierarchy, agent_tag }` | Park a message in the queue against an instance or a tag |
| `agents/get` | `path: RemotePathCommitOptional` | `{ path, inner: RemoteAgentBaseWithFallbacks }` | Fetch one agent definition |
| `agents/instances/get` | `targets: Vec<Target>` | stream of the `instances/list` item | Aggregates for exactly the named agents, zero-filled when idle |
| `agents/instances/list` | `targets`, `all: Option<bool>` | stream of `{ agent_instance_hierarchy, tags, queued, created_at, last_active_at, logged, laboratories, agent }` | List agent instances and their aggregates |
| `agents/list` | base only | stream of `RemotePath` | List the remote agents available from the configured source |
| `agents/logs/list` | `pending`, `targets`, `after_id`, `limit` | stream of log rows: `RequestMessageUser`, `RequestMessageAssistant`, `RequestMessageTool`, `VectorRequestChoices`, `VectorResponseVote`, `ClientNotification`, `AssistantResponse`, `ToolResponse`, `Error` | Page the persisted conversation log |
| `agents/logs/open` | `id: i64` | one of `AgentCompletionRequest`, `VectorCompletionRequest`, `FunctionExecutionRequest`, `Text`, `Image`, `Audio`, `Video`, `File`, `Error` | Open one log row's full content |
| `agents/logs/subscribe` | `targets`, `kinds: Option<KindFilter>`, `after_id`, `limit` | stream of `Item(row)` or `AgentsInactive` | Tail the log live until the agents go inactive |
| `agents/logs/token-usage/get` | `agent_instance_hierarchy` | `{ agent_instance_hierarchy, total_tokens }` | Total token usage for one hierarchy |
| `agents/logs/token-usage/subscribe` | `agent_instance_hierarchy`, `previous: Option<i64>` | stream of `Item(TokenUsage)` or `AgentsInactive` | Live token usage for one hierarchy |
| `agents/mcp/resources/list` | `response_id`, `params: ListResourcesRequest`, `name` | `ListResourcesResult` | List a live agent's aggregated MCP resources |
| `agents/mcp/resources/read` | `response_id`, `params: ReadResourceRequestParams` | `ReadResourceResult` | Read one MCP resource through a live agent |
| `agents/mcp/servers/list` | `response_id` | `ListServersResult` | List the upstream MCP servers a live agent holds |
| `agents/mcp/tools/call` | `response_id`, `params: CallToolRequestParams` | `CallToolResult` | Call an MCP tool through a live agent |
| `agents/mcp/tools/list` | `response_id`, `params: ListToolsRequest`, `name` | `ListToolsResult` | List a live agent's aggregated MCP tools |
| `agents/message` | `agent: AgentSelector`, `message: RequestMessage`, `dangerous_advanced: Option<{ seed }>` | `Delivered` or `Id { agent_instance_hierarchy }` | Unary delivery: enqueue and race an existing agent, or spawn a fresh one |
| `agents/publish` | `repository`, `body: RequestBody`, `message: RequestPublishMessage`, `overwrite` | `{ sha }` | Publish an agent definition |
| `agents/queue/delete` | `id` | `{ id, agent_instance_hierarchy, agent_tag, key, enqueued_at, content }` | Delete one queued message and return it |
| `agents/queue/deliver` | `keys: Option<Vec<String>>`, `dangerous_advanced` | stream of `Value`, `AgentActive`, `AgentSpawned`, `TagActive`, `TagSpawned`, `AllAgentsActive` | Drain the deferred queue, spawning or waking agents as needed |
| `agents/queue/list` | `targets`, `after_id`, `limit` | stream of `AgentInstanceHierarchy { .. }` or `Tag { .. }` | List queued messages by target |
| `agents/queue/open` | `id` | `RichContentPart` | Open one queued message's content |
| `agents/spawn` | `message`, `agent: AgentSelector`, `dangerous_advanced` | stream of `Id(String)` then `Chunk(AgentCompletionChunk)` | Spawn a streaming agent completion as a child of the caller |
| `agents/tags/apply` | `name`, `target: Target` | `AgentInstance { .. }`, `Agent { .. }` or `AgentTag { .. }` | Apply or move a tag |
| `agents/tags/lookup` | `by`: `agent_instance_hierarchy { parent_agent_instance_hierarchy, agent_instance }` or `tag { tag }` | `AgentInstanceHierarchy { tags }`, `Tag { .. }` or `Absent` | Resolve a tag to a hierarchy or a hierarchy to its tags |
| `agents/tags/remove` | `tag` | `{ name, removed, detached_laboratories }` | Remove a tag and detach the laboratories that travelled with it |
| `agents/wait` | `agent: AgentSelector`, `active: bool` | `Ok` | Block until the agent's lock chain releases, or the timeout |

### api/config

`objectiveai-sdk-rs/src/cli/command/api/config/`, handlers
`objectiveai-daemon/src/command/api/`.

| Kind | Request | Response | Purpose |
|---|---|---|---|
| `api/config/get` | base only | `{ address, objectiveai_authorization, openrouter_authorization, github_authorization, mcp_authorization, user_agent, http_referer, x_title, commit_author_name, commit_author_email }` | The whole API client configuration |
| `api/config/address/get` · `set` | `set`: `value: String` | `{ address }` · `Ok` | The API address |
| `api/config/backoff_max_elapsed_time_ms/get` · `set` | `set`: `value` | `{ backoff_max_elapsed_time_ms }` · `Ok` | The retry backoff ceiling |
| `api/config/commit_author_email/get` · `set` | `set`: `value` | `{ commit_author_email }` · `Ok` | The publish commit author's email |
| `api/config/commit_author_name/get` · `set` | `set`: `value` | `{ commit_author_name }` · `Ok` | The publish commit author's name |
| `api/config/github_authorization/get` · `set` | `set`: `value` | `{ github_authorization }` · `Ok` | The GitHub credential |
| `api/config/http_referer/get` · `set` | `set`: `value` | `{ http_referer }` · `Ok` | The outbound `HTTP-Referer` |
| `api/config/mcp_authorization/get` · `add` · `del` | `add`: `key`, `value`; `del`: `key` | `{ mcp_authorization: map }` · `Ok` · `Ok` | The per-MCP credentials |
| `api/config/mcp_call_timeout_ms/get` · `set` | `set`: `value` | `{ mcp_call_timeout_ms }` · `Ok` | The MCP call timeout |
| `api/config/mcp_connect_timeout_ms/get` · `set` | `set`: `value` | `{ mcp_connect_timeout_ms }` · `Ok` | The MCP connect timeout |
| `api/config/objectiveai_authorization/get` · `set` | `set`: `value` | `{ objectiveai_authorization }` · `Ok` | The ObjectiveAI API credential |
| `api/config/openrouter_authorization/get` · `set` | `set`: `value` | `{ openrouter_authorization }` · `Ok` | The OpenRouter credential |
| `api/config/user_agent/get` · `set` | `set`: `value` | `{ user_agent }` · `Ok` | The outbound `User-Agent` |
| `api/config/x_title/get` · `set` | `set`: `value` | `{ x_title }` · `Ok` | The outbound `X-Title` |

### channels

`objectiveai-sdk-rs/src/cli/command/channels/`, handlers
`objectiveai-daemon/src/command/channels/`, the live hub
`http/channel_routes.rs`.

| Kind | Request | Response | Purpose |
|---|---|---|---|
| `channels/publish` | `key`, `details: Value`, `message: String` | `{ channel_id, secret }`, after someone accepts | Offer a duplex channel on `GET /channels` and wait for the accept |
| `channels/close` | `channel_id`, `secret` | `{ channel_id }` | Close a channel, with either secret |
| `channels/logs/list` | `channel_id`, `secret`, `pending`, `after_id`, `limit` | `{ entries: Vec<ChannelLogEntry> }` | Page the channel's durable log |
| `channels/logs/open` | `channel_id`, `secret`, `entry_id` | `Entry { .. }` or `NotFound` | Open one log entry |
| `channels/logs/request` | `channel_id`, `secret`, `content: Value` | `Appended { id, timestamp }` or `ChannelClosed` | Append a request message |
| `channels/logs/reply` | `channel_id`, `secret`, `content: Value` | `Appended { id, timestamp }` or `ChannelClosed` | Append a reply message |
| `channels/logs/subscribe` | `channel_id`, `secret`, `after_id`, `limit` | stream of `Item(ChannelLogEntry)` or `ChannelClosed` | Tail the log until the close |

### daemon

`objectiveai-sdk-rs/src/cli/command/daemon/`, handlers
`objectiveai-daemon/src/command/daemon/`.

| Kind | Request | Response | Purpose |
|---|---|---|---|
| `daemon/spawn` | `dangerous_advanced: Option<{ foreground, .. }>` | stream of `{ ok }` | Ensure the resident daemon is up; `foreground: true` runs it in-process |
| `daemon/kill` | base only | `{ killed }` | Kill resident daemon processes |
| `daemon/config/get` | base only | `{ address, secret, signature }` | The daemon's own configuration |
| `daemon/config/set` | `value: Value` | `Ok` | Replace it; the live auth secret follows without a restart |
| `daemon/config/address/get` | base only | `{ address }` | The bind address |
| `daemon/config/secret/get` | base only | `{ secret }` | The auth secret |
| `daemon/config/signature/get` | base only | `{ signature }` | The derived signature |
| `daemon/config/refresh_secret_signature_pair` | base only | `{ secret, signature }` | Mint a fresh pair |

### db

`objectiveai-sdk-rs/src/cli/command/db/`, handlers
`objectiveai-daemon/src/command/db/`.

| Kind | Request | Response | Purpose |
|---|---|---|---|
| `db/query` | `query: String` | `{ command_tag, columns, rows, truncated }` | Run one SQL statement against the daemon's Postgres |
| `db/config/get` | base only | `{ address, user, password, database }` | The database connection configuration |
| `db/config/set` | `value: Value` | `Ok` | Replace it |
| `db/config/address/get` · `user/get` · `password/get` · `database/get` | base only | `{ address }` · `{ user }` · `{ password }` · `{ database }` | One field each |

### development

`objectiveai-sdk-rs/src/cli/command/development/`, handlers
`objectiveai-daemon/src/command/development/`.

| Kind | Request | Response | Purpose |
|---|---|---|---|
| `development/plugins/mcp/create` | `owner`, `name`, `version`, `path` | `{ owner, name, version, path, replaced }` | Register a local-source override for a plugin's MCP image |
| `development/plugins/mcp/delete` | `owner`, `name`, `version` | `{ owner, name, version, removed }` | Remove that override |
| `development/plugins/mcp/list` | base only | stream of `{ owner, name, version, path }` | List the MCP overrides |
| `development/plugins/mcp/reset` | `owner`, `name`, `version`, `caches: bool` | `{ owner, name, version, removed, caches_removed }` | Reset the plugin's built MCP image, and its caches if asked |
| `development/plugins/viewer/create` | `owner`, `name`, `version`, `path` | `{ owner, name, version, path, replaced }` | Register a local-source override for a plugin's viewer extension |
| `development/plugins/viewer/delete` | `owner`, `name`, `version` | `{ owner, name, version, removed }` | Remove that override |
| `development/plugins/viewer/list` | base only | stream of `{ owner, name, version, path }` | List the viewer overrides |
| `development/viewer/get` | base only | `{ path }` | The viewer-app-from-source path |
| `development/viewer/set` | `path` | `{ path, replaced }` | Point the viewer app at a source tree |
| `development/viewer/delete` | base only | `{ removed }` | Clear it |

### functions

`objectiveai-sdk-rs/src/cli/command/functions/`, handlers
`objectiveai-daemon/src/command/functions/`.

| Kind | Request | Response | Purpose |
|---|---|---|---|
| `functions/execute/standard` | `function: FunctionSpec`, `profile: ProfileSpec`, `input: RequestInput`, `continuation`, `split`, `invert`, `dangerous_advanced` | stream of `Id`, `AgentInstanceHierarchy`, `Chunk(FunctionExecutionChunk)` | Run a standard function execution |
| `functions/execute/swiss_system` | the same, plus `pool: Option<usize>`, `rounds: Option<usize>` | the same stream | Run a Swiss-system function execution |
| `functions/get` | `path: RemotePathCommitOptional` | `{ path, inner: FullRemoteFunction }` | Fetch one function definition |
| `functions/list` | base only | `{ items }` | List functions |
| `functions/publish` | `repository`, `body`, `message`, `overwrite` | `{ sha }` | Publish a function |
| `functions/profiles/get` | `path` | `{ path, inner: RemoteProfile }` | Fetch one profile |
| `functions/profiles/list` | base only | `{ items }` | List profiles |
| `functions/profiles/publish` | `repository`, `body`, `message`, `overwrite` | `{ sha }` | Publish a profile |

### laboratories

`objectiveai-sdk-rs/src/cli/command/laboratories/`, handlers
`objectiveai-daemon/src/command/laboratories/`, the registry
`http/websocket_laboratory.rs`.

| Kind | Request | Response | Purpose |
|---|---|---|---|
| `laboratories/create` | `kind`, `id`, `image: LaboratoryImage`, `mounts: Vec<Mount>`, `env: Vec<EnvVar>`, `cwd`, `machine`, `machine_state` | `{ id, image, mounts, env, cwd, created_at, machine, machine_state }` | Create a laboratory container on a host |
| `laboratories/delete` | `kind`, `id`, `machine`, `machine_state` | `{ id }` | Delete a laboratory |
| `laboratories/list` | `kind` | stream of `{ id, image, mounts, env, cwd, created_at, agent_full_id, plugin, response_id, machine, machine_state, running }` | The laboratories served by connected hosts |
| `laboratories/attach` | `selector: AgentSelector`, `laboratory_id`, `machine`, `machine_state` | `{ laboratory_id, machine, machine_state }` | Attach a laboratory to an agent instance or tag |
| `laboratories/detach` | the same | the same | Detach it |
| `laboratories/spawn` | base only | `{ addresses }` | Spawn the local laboratory host processes |
| `laboratories/kill` | base only | `{ killed }` | Kill them |
| `laboratories/config/local/get` · `set` | `set`: `value: bool` | `{ local }` · `Ok` | The local-host flag |
| `laboratories/config/addresses/get` · `add` · `del` | `add`: `key`, `value`; `del`: `key` | `{ addresses: map }` · `Ok` · `Ok` | The remote-host address book |

### swarms, tasks, python, update, viewer

`objectiveai-sdk-rs/src/cli/command/{swarms,tasks,python,update,viewer}/`.

| Kind | Request | Response | Purpose |
|---|---|---|---|
| `swarms/get` | `path` | `{ path, inner: RemoteSwarmBase }` | Fetch one swarm definition |
| `swarms/list` | base only | `{ items }` | List swarms |
| `swarms/publish` | `repository`, `body`, `message`, `overwrite` | `{ sha }` | Publish a swarm |
| `tasks/create` | `id`, `command: Box<Request>`, `delay_secs`, `repeat`, `repeat_count` | `{ id }` | Schedule any `/execute` request to run later, or repeatedly |
| `tasks/delete` | `id`, `namespace: DeleteNamespace` | `{ deleted }` | Delete a scheduled task |
| `tasks/list` | base only | stream of `{ id, command, delay_secs, repeat, repeat_count, run_count, error_count, last_result, complete, created_at, next_run_at, agent_instance_hierarchy, plugin_owner, plugin_name, plugin_version }` | List scheduled tasks and their state |
| `python` | `code`, `input: Option<Value>`, `no_objectiveai` | `Value` | Run a Python snippet in the daemon's interpreter |
| `update` | base only | stream of `Checking`, `Found`, `Installed`, `Skipped`, `UpToDate` | Self-update the binary |
| `viewer/spawn` | base only | `{ listening }` | Launch the viewer app |
| `viewer/kill` | base only | `{ killed }` | Kill viewer processes |

### schema introspection

For every leaf `P` above, `P/request_schema` and `P/response_schema`
take `{ path_type, ..base }` and answer `ResponseSchema`, the JSON
Schema of that leaf's request or of its response or item type. The
root-level `python` and `update` leaves carry theirs as the top-level
kinds `PythonRequestSchema`, `PythonResponseSchema`,
`UpdateRequestSchema` and `UpdateResponseSchema`.

## `/mcp` — the daemon's MCP server

`http/mcp/{mod,service,objectiveai,header_session_manager,agent_args_registry,format}.rs`.

Streamable HTTP, nested at `/mcp` on the daemon's own port; stateful
sessions; protocol version `2025-06-18`; server name `oai`; the tools
capability alone, no instructions.

| Exchange | Request | Response | Purpose |
|---|---|---|---|
| `initialize` | the `X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY`, `-AGENT-ID`, `-AGENT-FULL-ID`, `-AGENT-REMOTE`, `-RESPONSE-ID`, `-RESPONSE-IDS` headers, and `X-OBJECTIVEAI-MCP-ROOT` | `InitializeResult` | Bind the session to an identity and the root gate; a session is minted on any `POST`, and a reconnect with new headers replaces the identity whole |
| `tools/list` | optional pagination | `ListToolsResult` | The one tool, gated on the session's `mcp_root` |
| `tools/call` of `ObjectiveAI` | `{ command: Vec<String>, timeout, max_tokens, jq, python }` | `CallToolResult`, the run's items formatted as JSONL or text | The whole CLI as one tool: the argv is parsed into a `Request` and run through the executor |
| `GET /mcp` | `Mcp-Session-Id` | SSE | The server-to-client message channel |

## `/laboratory` — the one WebSocket

`http/websocket_laboratory.rs`; wire types
`objectiveai-sdk-rs/src/laboratories/daemon/{daemon,payload}.rs`.

The order is load-bearing: `HostIdentify` first, `AuthEnvelope`
second, then correlated request and response beside uncorrelated
notifications. The set of live connections is the laboratory
registry: a host that disconnects takes every laboratory it served.

| Frame | Shape | Purpose |
|---|---|---|
| host → daemon, first | `HostIdentify { state, machine: MachineIdentity, laboratories: Vec<Identify> }`, each `Identify { id, image, mounts, env, cwd, created_at, agent_full_id, plugin, response_id, running }` | Who the host is and the laboratories it serves |
| host → daemon, second | `AuthEnvelope { signature }` | The signature; the connection closes on a mismatch |
| daemon → host | `ChannelRequest { id, laboratory_id, headers, payload: RequestPayload }`, answered by `ChannelResponse { id, payload: ResponsePayload }` | One laboratory operation, correlated by `id`: `Initialize`, `SessionTerminate`, `ToolsList`, `ToolsCall`, `ResourcesList`, `ResourcesRead`, `Drop`, `Filetree`, `ExportBegin`, `ExportRead`, `ExportAbort`, `ImportBegin`, `ImportWrite`, `ImportEnd`, `ImportAbort`, `Create`, `AgentEphemeralCreate`, `PluginEphemeralCreate`, `PluginImageReset`, `Delete`, `LocalTransfer`, `BuildCreate`, `BuildRead`, `BuildAbort` |
| host → daemon, uncorrelated | `HostNotification`: `LaboratoryCreated`, `LaboratoryUpdated`, `LaboratoryDeleted`, `LaboratoryFiletree`, `McpListChanged { kind: Tools or Resources }` | Keep the daemon's view of the host's laboratories and file trees current |
| host → daemon, host-initiated | `HostCommandRequest { id, identity, plugin, request: Request }`, answered by a stream of `HostCommandResponse { id, frame }` with `frame` one of `Ack`, `Item`, `Error`, `Done` | Let a host run a CLI command on the daemon |
| both ways | `HostPostgres::Open { stream_id }`, `Close { stream_id }`, `PostgresData { stream_id, bytes }`; `DaemonPostgres::Close`, `PostgresData` | Tunnel raw Postgres bytes between a laboratory and the daemon's database, 64 KiB per frame |

## Behind the routes, with no endpoint of their own

| File | What it serves |
|---|---|
| `http/conduit.rs` | Handles the MCP requests the API pushes down the outbound agent-completion WebSocket — `Initialize`, `ToolsList`, `ToolsCall`, `ResourcesList`, `ResourcesRead`, `SessionTerminate`, `ReadMessageQueue`, `Retrieve`, `Script`, `Drop`, `Command`, the laboratory export, import and transfer set — forwarding each to a laboratory host, and registers the notifiers the `agents/mcp/*` kinds use |
| `http/mcp_listener.rs` | The in-process surface behind `agents/mcp/*`: `tools/list`, `tools/call`, `resources/list`, `resources/read`, `servers/list` |
| `http/agent_registry.rs` | Per-hierarchy exclusive claims and tag-lock families; what produces `Activated` and `Deactivated` on `/agents/instances/list` |
| `http/agent_hierarchies.rs` | The recursive hierarchy walk over completion chunks the per-chunk lock claim uses |
| `http/daemon_auth.rs` | The signature check on every route and on the `/laboratory` envelope |
| `http/agents_routes.rs`, `http/agent_instance_route.rs`, `http/laboratories_routes.rs`, `http/channel_routes.rs`, `http/plugin_routes.rs` | The handlers of the SSE and build routes above |
| `filesystem/` | The daemon's configuration and state files, reached through the `*/config/*`, `*/publish` and `development/*` kinds |
