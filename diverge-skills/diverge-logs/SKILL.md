---
name: diverge-logs
description: Observing a running Diverge daemon. Covers `diverge run logs` (the one log read for an agent, a standalone tool, a connected standalone tool, a dependency tool, an account's command log or an administrator's; index and time ranges, `type`, `count`, `watch`; what every log entry type means), `diverge run events list` (the graph of who sent to whom, parties and counts), `--watch` on list commands and the added / changed / removed / listed frames, `diverge run info`, and `--jq` recipes for common questions. Load it when asked what an agent said or did, whether an agent or tool is deployed or active, why a container stopped, who messaged whom, which tool calls were made, which requests an account made or was refused, to follow a log or a list live, to wait for an agent's reply, or to check that a daemon is up and which version it is.
---

# Diverge logs, events, watching

The daemon keeps three kinds of record of what happened, all read-only over the wire:

- **Logs**, one per agent, per tool and per requester. An agent's log is its whole conversation plus its
  container's life. A tool's log is every call and read routed into it. An account's (or administrator's)
  **command log** is every request it put to the daemon and whether it was let through.
- **Events**, one graph: an edge per (sender, receiver) pair with a count. It says that exchanges happened
  and how often, never what they carried or when.
- **Lists**, which every family has, and which can be kept open (`--watch`) to see records change.

Commands are `diverge run … '<JSON body>'`; each answer is one compact JSON line on stdout. Global options
go right after `run`: `--daemon`, `--account`, `--jq`, `--timeout`.

## How answers print (read this once)

- An answer with a payload is an object with one member named for the variant: `{"found":{…}}`,
  `{"added":{…}}`, `{"item":{…}}`, `{"error":…}`. An answer without one is a **bare string**: `"listed"`,
  `"watch"`, `"forbidden"`, `"not_found"`.
- `--jq '<filter>'` runs over every line. Each value the filter yields is printed as JSON on its own line; a
  line the filter yields nothing (or only `null`) for is not printed. Output is always JSON, so strings keep
  their quotes; strip them with `| tr -d '"'` or a local `jq -r .`.
- **Because some frames are bare strings, use `?`**: `--jq '.added? | .name'` (a bare `.added` errors on
  `"listed"`). Equivalent: `--jq 'objects | .added | …'`.
- A daemon-side failure is an answer, `{"error":{"kind":"daemon","error":"the agent named is none the daemon has"}}`,
  and the command still exits 0. Check the line, not the exit code. A connection, body or timeout problem
  is the CLI's own error on stderr with a non-zero exit.
- Every body may carry `"relay":["<daemon id>"]` to read another daemon's logs, events or lists
  (`diverge-daemons`).

## 1. `diverge run logs`

One request for every log. `of` names the subject; everything else narrows.

```bash
diverge run logs '{"of":{"type":"agent","name":"echo"}}'                       # the whole log, oldest first
diverge run logs '{"of":{"type":"agent","name":"echo"},"logs_index_from":120}'  # from entry 120 on
diverge run logs '{"of":{"type":"agent","name":"echo"},"type":"assistant_text_content","count":5}'
diverge run logs '{"of":{"type":"agent","name":"echo"},"created_from":"2026-11-10T06:00:00Z","created_to":"2026-11-10T07:00:00Z"}'
diverge run logs '{"of":{"type":"agent","name":"echo"},"logs_index_from":178,"watch":true}'   # then follow live
```

Answers: `{"item":{"<subject kind>":{"logs_index":…,"created":…,"type":…, …}}}` per entry;
`"watch"` once, between history and live entries, only when watching; `"forbidden"`; `{"error":…}` (a
subject that does not exist is an error, not `not_found`).

### Subjects (`of`)

`of` is `type` plus the members of that family's reference, flat:

| `of.type` | Rest of `of` | Log | Grant (`diverge-access`) |
|---|---|---|---|
| `agent` | `"name":…` or `"template":…,"index":…` | agent log | `logs` over agents |
| `standalone_tool` | `"name":…` or `"template":…,"index":…` | tool log | `logs` over standalone tools |
| `connected_standalone_tool` | `"name":…` or `"daemon":"<id>","tool":{…},"index":…` | tool log (calls this daemon routed into the remote tool) | `logs` over connected tools |
| `dependency_tool` | `"template":"<dependency template id>","owner":…` where owner is `{"agent":{"name":…}}`, `{"agent_template":{"template":…}}` or `"global"` | tool log | `logs` over dependency tools |
| `account` | `"issuer":…,"subject":…` | command log | `logs` over accounts |
| `admin` | `"issuer":…,"subject":…` | an administrator's command log | administrators only |

The `{"template","index"}` forms name a subject once and for all; a name is reusable after a delete. Logs
are removed with their subject.

### Filters

| Member | Meaning |
|---|---|
| `logs_index_from`, `logs_index_to` | Inclusive span of entry indexes. Indexes count up from 1 per log and never repeat. |
| `created_from`, `created_to` | Inclusive span of time, RFC 3339 UTC (`2026-11-10T06:00:00Z`). |
| `type` | One entry type by name (table below). A name the log read does not have matches nothing. |
| `count` | At most this many entries sent, counted after filtering, oldest first. |
| `watch` | `true`: send what matches now, then `"watch"`, then each new matching entry as it lands. |

All are optional and combine (every one given must hold). There is no "last N" or reverse order: to read
the tail, get the log's latest index first (an agent's `get`/`list` item carries `logs_index`) and read
from `logs_index_from`.

A watch ends on its own when `count` is met, when the log reaches `logs_index_to`, or when the clock
passes `created_to`. With none of those it runs until the command is ended (or the subject is deleted).
**Always give a watch an end**: a `count`, a `logs_index_to`, a `created_to`, or `--timeout <seconds>` on
the command (a timeout ends it with an error on stderr; what was printed stays printed).

### What an agent log holds

Every entry has `logs_index`, `created` and `type`. The common ones:

| `type` | Meaning | Members worth reading |
|---|---|---|
| `deployed` | The agent's container came up on a provider. First entry of every run. | `identity` (the provider) |
| `active` / `inactive` | The agent began / ceased working (a loop running or an MCP exchange in flight). Alternate inside a run. The idle clock runs from `inactive`. | |
| `undeployed` | The container went down. Last entry of a run. | `identity`, `interrupted` (was it working), `reason` only when unexpected |
| `user_text_content` | One text part of a message that landed. | `text`, `key` (the message's id), `sender` |
| `assistant_text_content` | Text the agent said. | `text` |
| `assistant_reasoning` | The model's reasoning. | `text` |
| `assistant_tool_call` | The agent called a tool. | `id`, `name`, `arguments` |
| `tool_response` | That call's result. | `id` (the call's), `content`, `isError` |
| `assistant_refusal` | The model declined. | `text` |
| `user_structured_decisions` | A structured decision that landed, whole. | `id` (the log's number for it), `key`, `content`, `questions`, `sender` |
| `structured_decisions_response` | The agent's answers to one. | `id`, `decisions`: question id → `{"noul":n}`, `{"choice":value,"probabilities":{value:n}}` or `{"score":n,"probabilities":[n…]}`. `choice` (highest probability, alphabetically first on a tie) and `score` (sum of probability × level position, first level 0) are the daemon's, computed as the item is written |
| `usage` | Token usage so far. | `prompt_tokens`, `completion_tokens`, `total_tokens` |
| `notification` | Something about the run itself. | `message`, `is_fatal` |
| `error` | A run that would not start, or a message the agent refused. | `error` (free-form) |

Also `user_image_content`, `user_audio_content`, `user_resource`, `user_resource_link`,
`assistant_image_content`, `assistant_audio_content`. The complete tables for all three logs, with every
member and every `undeployed` reason, are in [references/log-entries.md](references/log-entries.md).

Reading rules:

- A run reads `deployed`, then `active … inactive` stretches, then `undeployed`. An `undeployed` with no
  `reason` was expected (idle timeout after `idle_seconds`, or a delete); the agent starts again on its
  next message. A run that never came up leaves an `error` and no `deployed`.
- Only `user_*` entries carry `sender`: `{"type":"client","identity":{…}}` for an account or
  administrator, `{"type":"agent",…}` or `{"type":"tool",…}` for a message sent through the daemon.
- A message is the run of `user_*` parts sharing one `key`. Messages that were waiting while the agent
  started are joined into its first turn, so several keys can precede one reply.
- A reply may come as several `assistant_text_content` entries; join the texts between an `active` and the
  next `inactive`. Entries with `parent_tool_call_id` belong to a sub-agent spawned by that tool call.
- **Matching a decision to its answers**: both carry a numeric `id` the daemon assigns as it writes. A
  decision takes the next id (one more than the highest before it, `0` first). The first response after a
  decision takes that decision's id. Any other response (a second one, or one with no decision before it)
  takes the next id. So `decision, response, response, decision, response` reads `0, 0, 1, 2, 2`: a
  response answers the decision with the same `id`, and a response whose id no decision has was
  unprompted. `"type":"structured_decisions_response"` in the logs request narrows to the answers.
- `_meta["diverge.network"]` on a chunk is the daemon's attestation of which agent spoke.

### What a tool log holds

`deployed`, `active`, `inactive`, `undeployed` (same meaning; `active` = an ask in flight); `call`
(`caller`, `name`, `arguments`) answered exactly once by `call_result` (`call` = the call's `logs_index`,
`result`), `call_error` (`call`, `error`) or `call_interrupted` (`call`); `read` (`caller`, `uri`) answered
by `read_result`, `read_error` or `read_interrupted` (each with `read`); `notification` (`to`,
`notification`); `error` (the daemon's own); `command` (a request the tool's container put to the daemon:
`path`, `request`, `allowed`). `caller` is `{"kind":"agent","template":…,"index":…,"name":…,"image":…}`
or `{"kind":"client","identity":{…},"scope":…}`. `tools/list` and `resources/list` are not logged.

### What a command log holds

One entry type, `command`: `path` (`agents/message`, `roles/create`, `logs`), `request` (the body as
sent, `relay` included, secrets replaced), `allowed` (`false` exactly when the answer was `forbidden`; a
request that was let through and then failed is still `true`). One entry per request; what the request
came to is not kept. Requests the daemon could not parse are not kept.

## 2. `diverge run events list`

```bash
diverge run events list
# {"added":{"id":65,"sender":{"kind":"account","issuer":"https://identity.diverge.network","subject":"github.12345.admin"},
#           "receiver":{"kind":"agent","template":"b6dd4f61…","index":1},"count":85}}
# "listed"
diverge run events list '{"count":20}'                       # the LATEST 20 edges by id
diverge run events list '{"parties":[{"kind":"agent","template":"b6dd4f61…","index":1}]}'   # everything touching one agent
diverge run events list '{"senders":[{"kind":"account","issuer":"https://identity.diverge.network","subject":"github.12345.alice"}]}'
diverge run events list --watch                              # then {"changed":{…}} each time a count rises
```

- **An event is one edge**: `id`, `sender`, `receiver`, `count`. The first occurrence adds the edge with
  `count` 1; each later one raises `count`. A thousand messages between one pair is one event.
- **What creates an occurrence**: a message enqueued to an agent (receiver is an agent), an MCP tool call
  into a tool, an MCP resource read into a tool (receiver is a tool). The event does not say which; the
  receiver's kind implies it. Nothing else is recorded: no body, no arguments, no result, no time.
- **Parties** are named by what never changes, never by name:
  `{"kind":"account","issuer":…,"subject":…}` (an account or an administrator),
  `{"kind":"agent","template":…,"index":…}`, `{"kind":"standalone_tool","template":…,"index":…}`,
  `{"kind":"dependency_tool","template":…,"index":…}`. A connected tool is never a party: it stands as the
  party that connected it. To turn a party into a name, match `template` and `index` against the family's
  list.
- **Filter**: `parties` (either side), `senders`, `receivers`; every member given must hold. `count`
  bounds only the initial listing, to the latest that many by id; what arrives after `"listed"` is uncapped.
- An edge is removed when either party is deleted (`{"removed":{…}}` when watching).
- Grant: `{"events":["list"]}`. The graph is one thing: an account that may list it sees all of it.

For "when" and "what", go to the logs: the agent's log has the `user_*` entries with `sender` and time, a
tool's log has each `call` with its `caller`.

## 3. `--watch` on lists

Without `--watch`, a list prints each record as `{"added":{…}}`, then `"listed"`, and the command ends.
With `--watch` the command stays open after `"listed"` and prints every change until it is ended:

| Frame | Meaning |
|---|---|
| `{"added":{…}}` | A record that matches, listed now (initially, or newly matching). |
| `{"changed":{…}}` | A record already listed, whole, as it is now. |
| `{"removed":{…}}` | A record already listed that no longer matches or was deleted, as last sent. |
| `"listed"` | The initial listing is complete. Sent once. |
| `"forbidden"` / `{"error":…}` | No `list` grant over the kind at all / a failure; the list ends. |

Leaves with `--watch`:

- Lists: `accounts list`, `roles list`, `agents list`, `agents templates list`, `standalone-tools list`,
  `standalone-tools templates list`, `connected-standalone-tools list`, `dependency-tools list`,
  `providers list`, `providers daemons list`, `daemons list`, `volumes list`, `vault list`, `postgres list`,
  `events list`.
- Filetrees: `agents filetree`, `standalone-tools filetree`, `dependency-tools filetree` (they end at the
  first snapshot without `--watch`; see `diverge-agents` and `diverge-tools`).

Not watchable this way: `identities list` (one snapshot answer), `logs` (use `"watch":true` in the body),
`providers daemons listen` (always open).

What counts as a change depends on the record: an agent's `active`, `deployed`, `last_active`,
`logs_index`; an account's `connected`; a provider's `status` and `capacity`; an event's `count`. The
list's own filter and `count` apply throughout: with `count`, the list is kept to the first that many
matches, so one leaving the window is `removed` as another is `added` (events differ: see above).

A watch never ends by itself. Run it for a program that reads the stream and ends the command, or bound it:

```bash
# Print until the agent becomes inactive, then stop (at most 120 s).
diverge run --timeout 120 agents list --watch '{"names":["echo"]}' \
  --jq '(.added?, .changed?) | select(. != null) | {active, deployed, logs_index}'
```

## 4. `diverge run info`

```bash
diverge run info                              # {"info":{"version":"2.3.0"}}
diverge run --daemon b info
diverge run info '{"relay":["<daemon id>"]}'  # the remote's version: also tests the relay
```

One answer: the daemon's version, which is also the protocol revision (daemon, provider and SDK are
versioned together). It needs no grant: any admitted connection is answered, so `info` is the check for
"is the daemon up, and am I admitted". If it fails to connect, the daemon is down or the address in
`cli.yaml` is wrong; if the connection is closed with no answer, the identity is not admitted
(`diverge-access`). A provider whose `status.type` is `different_version` in `providers list` is not at
this version (`diverge-daemons`).

## 5. `--jq` recipes

```bash
# Is this agent deployed / active right now?
diverge run agents get '{"agent":{"name":"echo"}}' --jq '.found | {active, deployed, last_active, last_deployed, logs_index}'

# All agents at a glance
diverge run agents list --jq '.added? | {name, active, deployed, last_active}'

# What did it last say?  (latest index from the agent, then read the tail)
L=$(diverge run agents get '{"agent":{"name":"echo"}}' --jq '.found.logs_index')
diverge run logs '{"of":{"type":"agent","name":"echo"},"logs_index_from":'$((L>20 ? L-20 : 1))',"type":"assistant_text_content"}' \
  --jq '.item.agent.text' | tail -1

# The last stretch of the conversation, compact
diverge run logs '{"of":{"type":"agent","name":"echo"},"logs_index_from":'$((L>30 ? L-30 : 1))'}' \
  --jq '.item.agent | [.logs_index, .type, (.text // .name // .reason // "")] | join(" ")'

# Send a message and print the reply: "watch" streams the log entries of that message's run, from the
# message itself to the run's `inactive`, and exits (see `diverge-agents`, Watching)
diverge run --timeout 300 agents message '{"agent":{"name":"echo"},"type":"chat","watch":true,"content":[{"type":"text","text":"hello"}]}' \
  --jq '.item.agent | select(.type=="assistant_text_content") | .text'

# Who messaged this agent, when, and what
diverge run logs '{"of":{"type":"agent","name":"echo"},"type":"user_text_content"}' \
  --jq '.item.agent | {created, from: (.sender.identity.subject // .sender.name // .sender.type), text}'

# Who messaged whom (the graph)
diverge run events list --jq '.added? | "\(.sender.kind):\(.sender.subject // .sender.index) -> \(.receiver.kind):\(.receiver.index) x\(.count)"'

# Which tools did the agent call, with arguments and results
diverge run logs '{"of":{"type":"agent","name":"echo"}}' \
  --jq '.item.agent | select(.type=="assistant_tool_call" or .type=="tool_response") | {logs_index, type, id, name, arguments, isError}'

# Every call into a tool, and who made it
diverge run logs '{"of":{"type":"standalone_tool","name":"notebook"},"type":"call"}' \
  --jq '.item.standalone_tool | {logs_index, created, tool: .name, by: (.caller.name // .caller.identity.subject), arguments}'

# Calls that failed or were cut off
diverge run logs '{"of":{"type":"standalone_tool","name":"notebook"}}' \
  --jq '.item.standalone_tool | select(.type=="call_error" or .type=="call_interrupted")'

# Why did the container stop?  (no reason = expected: idle or deleted)
diverge run logs '{"of":{"type":"agent","name":"echo"},"type":"undeployed"}' --jq '.item.agent | {created, reason, interrupted}'

# Errors in a log
diverge run logs '{"of":{"type":"agent","name":"echo"},"type":"error"}' --jq '.item.agent | {created, error}'

# Token usage, latest
diverge run logs '{"of":{"type":"agent","name":"echo"},"type":"usage"}' --jq '.item.agent | {prompt_tokens, completion_tokens, total_tokens}' | tail -1

# What an account did, and what it was refused
diverge run logs '{"of":{"type":"account","issuer":"https://identity.diverge.network","subject":"github.12345.alice"}}' \
  --jq '.item.account | {created, path, allowed}'
diverge run logs '{"of":{"type":"account","issuer":"https://identity.diverge.network","subject":"github.12345.alice"}}' \
  --jq '.item.account | select(.allowed == false) | {created, path, request}'

# My own requests as an administrator in the last hour
diverge run logs '{"of":{"type":"admin","issuer":"https://identity.diverge.network","subject":"github.12345.admin"},"created_from":"2026-11-10T06:00:00Z"}' \
  --jq '.item.admin | select(.path=="agents/message") | {created, to: .request.agent, allowed}'

# Is the provider connected?
diverge run providers list --jq '.added? | {subject: .identity.subject, status: .status.type}'
```

Notes on the recipes: in `--jq`, `.item.<kind>` uses the same word as `of.type`. `tail -1` and `sort | uniq -c`
are the shell's, applied after the CLI has printed. Shell arithmetic such as `$((L-20))` needs `L` to be a
bare number, which `.found.logs_index` prints.

## Pitfalls

- A subject that does not exist answers `{"error":…}`, exit code 0.
- A relayed read needs the grants on the remote, as the account the record names.
- Reading a log does not start a container, and a watch on a stopped agent simply waits.
- `events` has no times and no content; do not try to reconstruct a conversation from it.
- Command logs include the `logs` requests themselves, so reading a busy administrator's own log grows it.

## Not verified

- Run live while writing this (read-only, as an administrator): `info`; `logs` on an agent with
  `logs_index_from`, `logs_index_to`, `type`, `count`, `created_from` and `watch`; an administrator's
  command log; `events list` with and without `receivers`; `agents list|get`; the `.added?` jq form. Entry
  shapes shown for agents, command logs and events are real output.
- **`count: 0`**: the SDK says it sends nothing; the installed daemon answered one entry for
  `{"count":0}` on `logs`. Do not rely on `0`; use `count >= 1`.
- Tool-log entries (`call`, `read`, `notification`, tool `command`), `account`-typed command logs,
  `connected_standalone_tool` and `dependency_tool` subjects, and `removed` frames were not observed live;
  their shapes are from the SDK types and the CLI's `response-schema`.
- Whether an assistant reply is split into several `assistant_text_content` entries depends on the agent
  image; the one observed (the python loop) wrote one entry per reply.

## When this task is done

Load the `diverge` skill again before starting the next Diverge task: it is the index, and it says which
skill the next task needs.
