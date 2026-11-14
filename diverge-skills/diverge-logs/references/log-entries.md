# Every log entry type

Every line of `diverge run logs` that carries an entry is

```json
{"item":{"<subject kind>":{"logs_index":177,"created":"2026-11-10T06:33:05.628392800Z","type":"<type>", …}}}
```

`<subject kind>` is the request's `of.type` (`agent`, `standalone_tool`, `connected_standalone_tool`,
`dependency_tool`, `account`, `admin`). `logs_index` counts up from 1 within that one log and never
repeats. `created` is when the daemon kept the entry. The entry's own members lie flat beside those two.
The request's `type` filter takes any name below; a name the log being read does not have matches nothing.

For exact shapes: `diverge run logs response-schema --jq '."$defs" | keys'`.

## The run's life (agent logs and tool logs alike)

Only the daemon writes these. A log reads `deployed`, zero or more `active … inactive` stretches,
`undeployed`, and again for the next run.

| `type` | When | Members |
|---|---|---|
| `deployed` | The container is up on a provider. Before anything the run does. | `identity`: the provider, `{"issuer":…,"subject":…}` |
| `active` | The run began working. Agent: a loop runs in it, or an MCP exchange is in flight for it (its own, a dependency's, or one run as it through `agents connect`). Tool: an ask came in while none was in flight. Kept only on a change. | none |
| `inactive` | The run ceased working. Never kept after `undeployed`. The idle clock (`idle_seconds`) runs from here. | none |
| `undeployed` | The container is down, however it went down. | `identity` (the provider), `interrupted` (true if it was working; an `inactive` then precedes this), `reason` (only when unexpected) |

`undeployed.reason`, present only for an end nobody asked for:

| `reason` | Meaning |
|---|---|
| `provider_lost` | The connection to the provider the run was on ended. |
| `provider_disabled` | The provider was disabled (`providers disable`). |
| `provider_edited` | The provider's record was edited so that its `ips` no longer admit the connected provider's source address, which closes the connection. No other edit does. |
| `daemon_stopped` | The daemon stopped. |
| `mount_lost` | A FUSE mount of the run's lost its serve: the provider holding the mounted volume went, or the connection to the daemon the volume is on went, at any hop. |
| `exited` | The container exited on its own. |
| `failed` | The run's stream failed; the `error` entry kept just before says how. |

No `reason` means the run was taken down on purpose: gone idle, its record deleted, or (a dependency
service) retired. A run that never came up keeps an `error` and no `deployed`. Work cut short because a
connection under it went is not an `inactive`: the run stays `active` until proven alive or until its
`undeployed` says `interrupted`.

## Agent log

### User parts (a message that landed)

One entry per content block of the message, in order. Each carries `key`, the message's id (a UUID): the
parts of one message share it. Each carries `sender`, and nothing else in the log does.

| `type` | Members |
|---|---|
| `user_text_content` | `text`, `key`, `sender` |
| `user_image_content` | `data` (base64), `mimeType`, `key`, `sender` |
| `user_audio_content` | `data` (base64), `mimeType`, `key`, `sender` |
| `user_resource` | `resource` (an embedded MCP resource), `key`, `sender` |
| `user_resource_link` | `uri`, `name`, optional `title`, `description`, `mimeType`, `size`, `icons`, `key`, `sender` |

`sender` is who sent the message (the message request names no sender; the daemon knows from the
connection):

- `{"type":"client","identity":{"issuer":…,"subject":…}}`: an account or administrator, with `agents message`.
- `{"type":"agent","template":…,"index":…,"name":…}`: another agent, through the daemon.
- `{"type":"tool","record":{"template":…,"index":…,"name":…}}` or
  `{"type":"tool","dependency":{"template":…,"owner":…}}`: a tool, through the daemon.

### What the agent said and did

| `type` | Meaning | Members |
|---|---|---|
| `assistant_text_content` | Text from the model. | `text`, optional `logprobs`, `annotations` |
| `assistant_reasoning` | The model's reasoning. | `text` |
| `assistant_image_content` | An image from the model. | `data`, `mimeType` |
| `assistant_audio_content` | Audio from the model. | `data`, `mimeType` |
| `assistant_tool_call` | The model calling a tool. | `id`, `name` (as the agent sees it, prefixed), `arguments` |
| `assistant_refusal` | The model declining. | `text` |
| `tool_response` | A tool call's result. | `id` (the call's `id`), `content`, `structuredContent`, `isError` |
| `usage` | Token usage so far. | `prompt_tokens`, `completion_tokens`, `total_tokens` |
| `notification` | Something about the run itself. | `message`, `is_fatal` |
| `error` | The run that would not start, or the message the agent refused, in the words the daemon received. | `error`: an arbitrary JSON value |

- The six assistant types and `tool_response` may carry `parent_tool_call_id`: absent, the main thread;
  present, the `id` of the tool call whose sub-agent produced the entry.
- Every chunk carries `_meta`, where `_meta["diverge.network"]` is the daemon's attestation of the
  speaking agent: `{"kind":"agent","image":{…},"template":…,"index":…,"name":…}`.
- A `tool_response` pairs with the `assistant_tool_call` of the same `id`.
- Text entries are whole strings as the agent's program streamed them; a program may stream one reply as
  several entries.

## Tool log (standalone, connected, dependency)

The daemon's own record of every exchange it routed into the tool; never the tool's word.

| `type` | Meaning | Members |
|---|---|---|
| `deployed`, `active`, `inactive`, `undeployed` | The run's life, as above. For a connected tool, `deployed` means this daemon holds the other daemon's connect. | as above |
| `call` | A `tools/call` routed into the tool. Kept before the tool is asked. | `caller`, `name` (the tool's own name for it, not the prefixed one the agent saw), `arguments` |
| `call_result` | The tool's answer, whole (an answer the tool marked an error included). | `call` (the call's `logs_index`), `result` (`content`, `structuredContent`, `isError`) |
| `call_error` | The tool did not answer: it could not be started, or the exchange failed. | `call`, `error` |
| `call_interrupted` | The call was still open when the run went down; the caller got an error. | `call` |
| `read` | A `resources/read` routed into the tool. | `caller`, `uri` |
| `read_result` | The tool's answer. | `read` (the read's `logs_index`), `result` (`contents`) |
| `read_error` | The tool did not answer, or answered with an error. An agent's read is asked of each served tool in turn; each one asked keeps its own `read`, and each that did not hold the resource its own `read_error`. | `read`, `error` |
| `read_interrupted` | The read was still open when the run went down. | `read` |
| `notification` | One notification the tool sent, as MCP JSON. | `to` (the caller whose session it was sent on), `notification` (`method`, `params`) |
| `error` | An error of the daemon's own, met keeping the tool. Never an ask's answer. | `error` |
| `command` | A request the tool's own container put to the daemon. | `path`, `request`, `allowed` |

- Every `call` and `read` is answered exactly once in the log, by the entry whose `call` / `read` equals
  its `logs_index`.
- `caller` is `{"kind":"agent","image":…,"template":…,"index":…,"name":…}` for an agent's container, or
  `{"kind":"client","identity":{"issuer":…,"subject":…},"scope":"<uuid>"}` for a client reaching the tool
  over `connect` (another daemon holding the tool as a connected tool appears this way, as the account it
  connects as).
- `tools/list` and `resources/list` are neither logged nor counted as activity.

## Command log (`account`, `admin`)

| `type` | Members |
|---|---|
| `command` | `path`: the request as the daemon names it (`agents/message`, `vault/insert`, `logs`); `request`: the body as sent, `relay` and all, with secret values replaced (a vault insert's `value`, a login's password); `allowed`: `false` exactly when the daemon answered `forbidden` |

- One entry per request. Channels opened inside a request (a cancel, a disconnect, a connect's MCP
  exchanges) are not commands.
- A request that was let through and then came to `not_found` or an error is `allowed: true`; the outcome
  is not kept.
- A request the daemon could not read is not kept.
- An account's log is read with the `logs` grant over accounts reaching that account. An administrator
  with no account has its log under `{"type":"admin",…}`, read by administrators only.
- A tool container's own requests are in that tool's log as `command` entries, not here.

## Type names accepted by the `type` filter

`user_text_content`, `user_image_content`, `user_audio_content`, `user_resource`, `user_resource_link`,
`assistant_reasoning`, `assistant_text_content`, `assistant_image_content`, `assistant_audio_content`,
`assistant_tool_call`, `assistant_refusal`, `tool_response`, `usage`, `notification`, `error`, `deployed`,
`active`, `inactive`, `undeployed`, `call`, `call_result`, `call_error`, `call_interrupted`, `read`,
`read_result`, `read_error`, `read_interrupted`, `command`.
