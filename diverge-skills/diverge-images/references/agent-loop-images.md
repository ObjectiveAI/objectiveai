# The agent-loop images: python, openrouter, codex, eliza, cc

Reference for five of the six agent-loop images (hermes is in [hermes.md](hermes.md)). For each:
what it is, its `arguments`, the secrets its program declares and their modes, its dependencies, and
a minimal working **agent template body** for `diverge run agents templates create`.

Conventions used below:

- `<digest>` is the image's manifest digest as read from the provider's podman
  (`podman image inspect --format '{{.Digest}}' <name>`).
- Every body carries a `references` entry naming a registry the provider lists (`docker.io` by
  default), which the daemon requires even for a locally built image.
- `memory` and `disk` are **bytes** and are required on every template. The values shown are starting
  points, not measured minimums.
- A template's `secrets` is a list of **names**. The agent made from it binds each name to a vault
  key: `diverge run agents create '{"template":"<id>","name":"x","secrets":{"<NAME>":"<vault key>"}}'`.
  The create must bind every name the template declares and no other.
- The authoritative schema of an image's arguments is its `DIVERGE.md` frontmatter:
  `diverge run images info --jq '.info.schema' '{"image":{…}}'`.

## The `dependencies` member (every agent-loop image)

Every agent-loop image accepts an optional `dependencies` argument: the tool containers the agent
depends on. The program answers its registration with this list unchanged; the daemon deploys each
entry and serves it to the agent as an MCP server. Absent is none.

One entry:

```json
{
  "image": { "digest": "sha256:…", "references": [{ "registry": "docker.io", "name": "diverge-local/memory" }] },
  "memory": 268435456,
  "disk": 268435456,
  "arguments": {},
  "secrets": [],
  "mode": "transient"
}
```

| member | meaning |
|---|---|
| `image` | the tool's image, same shape as everywhere |
| `memory`, `disk` | the tool container's limits, in bytes |
| `arguments` | handed to the tool's program verbatim; what it may be is the tool image's `DIVERGE.md` schema |
| `secrets` | names the tool image needs (absent when none). Each must also be a name the **agent's template** declares; the value comes through the vault key the agent binds that name to |
| `mode` | `"transient"` or `"service"`, with that mode's own members flat beside it |

`"mode":"transient"` — deployed when the agent's container starts and stopped with the agent's run.
Its database scope is one per parent agent. It alone may mount the agent's own files, live over FUSE:

```json
"mode": "transient",
"fuse_file_mounts":      [{ "agent_path": ["root", "notes.md"], "tool_path": ["data", "notes.md"] }],
"fuse_directory_mounts": [{ "agent_path": ["workspace"],        "tool_path": ["workspace"] }]
```

`"mode":"service"` with `"scope"` — made once when the first agent that declares it registers, kept
running while any agent is attached, and retired after the daemon's idle time once none is. It mounts
nothing of any agent's. `scope` says how far it, and its database, is shared:

| `scope` | one service per |
|---|---|
| `"instance"` | parent agent |
| `"template"` | parent agent template (every agent of that template shares it) |
| `"global"` | dependency template, full stop (every agent that declares it, from any template) |

Rules:

- A dependency has no name. It is identified by the hash of its entry (image digest without
  references, limits, arguments, sorted secrets, mode and its members). Declaring the same entry twice
  in one list is refused. The same service entry under the same scope key is the same service.
- What a dependency tool may ask of the daemon (its permissions) is **not** in the entry: the tool's
  own program declares it at its registration. Some committed `DIVERGE.md` schemas still show a
  `permissions` member on dependency templates; the current SDK type has none.
- Only agents have dependencies. A tool container's own list is ignored.

## python (`diverge-agents/python`)

An agent that runs Python source instead of calling a model. Each turn runs the source against the
conversation; nothing is sampled. Useful for deterministic agents, echo agents and tests.

`arguments`:

| member | required | meaning |
|---|---|---|
| `python` | yes | the source, verbatim (never trimmed; whitespace is significant). Written to disk at registration and run every turn |
| `requirements` | no | third-party packages: a map of distribution name to `{"operator": "==" \| "!=" \| ">=" \| ">" \| "<=" \| "<" \| "~=", "version": "<PEP 440 version>"}`. Installed once at registration with pip. One constraint per package; compound ranges and environment markers are not representable |
| `dependencies` | no | see above |

The script contract (`diverge-agents/python/HARNESS.md`): the harness supplies the globals
`input` (the conversation; the last item is the newest message, a list of content blocks, or a
`tool_response`), `tools` and `resources`. The script's last bare expression (or else its printed
JSON) must be a list of assistant chunks, for example
`[{"type":"assistant_text_content","text":"…"}]` or
`[{"type":"assistant_tool_call","id":"call_…","name":"<tool>","arguments":"<json string>"}]`. A
script may never emit `user_*`, `usage` or `tool_response` chunks.

Secrets: **none**. The template need declare no name.

Minimal template body (an echo agent):

```json
{
  "type": "agent",
  "image": { "digest": "<digest>", "references": [{ "registry": "docker.io", "name": "diverge-local/python" }] },
  "memory": 536870912,
  "disk": 1073741824,
  "arguments": {
    "python": "last = input[-1] if input else []\nsaid = ' '.join(b.get('text', '') for b in last if isinstance(b, dict) and b.get('type') == 'text') if isinstance(last, list) else ''\n[{'type': 'assistant_text_content', 'text': 'echo: ' + said}]\n"
  }
}
```

Tip: build the body with a small script that reads the source from a file and JSON-encodes it, rather
than hand-escaping Python inside JSON inside a shell string.

With a requirement: `"requirements": {"requests": {"operator": ">=", "version": "2.31.0"}}`. A failed
install is a registration error (`kind: "requirements"`), surfaced as the agent's start error.

## openrouter (`diverge-agents/openrouter`)

An OpenRouter chat-completions loop. Its tools are the caller's MCP servers (attached tools and
dependencies).

`arguments`:

| member | required | meaning |
|---|---|---|
| `model` | yes | the model to route to, in OpenRouter's `vendor/name` form |
| `system_prompt` | no | sent as the conversation's leading message |
| `temperature`, `top_p`, `top_k`, `top_a`, `min_p` | no | sampling |
| `frequency_penalty`, `presence_penalty`, `repetition_penalty` | no | penalties |
| `max_tokens` | no | cap on generated tokens, excluding reasoning tokens |
| `max_completion_tokens` | no | cap on generated tokens, counting reasoning tokens |
| `stop` | no | a string or a list of strings |
| `logit_bias` | no | map of token id to integer bias |
| `top_logprobs` | no | alternatives to report per token; absent reports no log probabilities |
| `reasoning` | no | `{"enabled": bool, "max_tokens": n, "effort": "none"\|"minimal"\|"low"\|"medium"\|"high"\|"xhigh", "summary_verbosity": "auto"\|"concise"\|"detailed"}`, every member optional |
| `verbosity` | no | `"low"`, `"medium"`, `"high"` or `"max"` |
| `provider` | no | routing preferences: `{"allow_fallbacks": bool, "require_parameters": bool, "order": [...], "only": [...], "ignore": [...], "quantizations": [...]}`, every member optional |
| `context_compression` | no | `"middle_out"` |
| `dependencies` | no | see above |

Secrets: **`OPENROUTER_API_KEY`, fixed** — always, whatever the arguments. The value must be a JSON
string (the bearer token). The template must declare the name; a missing or non-string value fails
the run with an error naming the secret.

Minimal template body:

```json
{
  "type": "agent",
  "image": { "digest": "<digest>", "references": [{ "registry": "docker.io", "name": "diverge-local/openrouter" }] },
  "memory": 536870912,
  "disk": 1073741824,
  "arguments": { "model": "<vendor/model>", "system_prompt": "You are a concise assistant." },
  "secrets": ["OPENROUTER_API_KEY"]
}
```

```bash
diverge run vault insert '{"key":"or-key","value":"sk-or-…"}'
diverge run agents create '{"template":"<id>","name":"helper","secrets":{"OPENROUTER_API_KEY":"or-key"}}'
```

## codex (`diverge-agents/codex`)

An OpenAI Codex CLI agent, pre-installed in the image: one `codex exec` per turn on a resumed thread.
The container is the sandbox: Codex always runs with full access and never asks for approval. Its one
MCP server is the caller's tools through the proxy.

`arguments`:

| member | required | meaning |
|---|---|---|
| `model` | yes | the model, in the endpoint's own naming |
| `auth` | yes | how Codex logs in: an object tagged `method` (below). No default, because every method bills someone |
| `effort` | no | `"minimal"`, `"low"`, `"medium"`, `"high"`, `"xhigh"` |
| `reasoning_summary` | no | `"auto"`, `"concise"`, `"detailed"`, `"none"` — what the stream's reasoning chunks carry |
| `verbosity` | no | `"low"`, `"medium"`, `"high"` |
| `web_search` | no | `"disabled"` (the default when absent), `"cached"`, `"indexed"`, `"live"`. Codex searches itself; it is not one of the caller's tools |
| `provider` | no | `{"base_url": "https://…/v1"}`: an endpoint other than OpenAI's, which must speak the Responses API. **Requires `auth.method` = `api_key`**; any other method beside a provider is refused at registration |
| `dependencies` | no | see above |

Secrets: **exactly one, chosen by `auth.method`**. The template must declare that name.

| `auth` | secret name | mode | value |
|---|---|---|---|
| `{"method":"api_key"}` | `OPENAI_API_KEY` | fixed | a JSON string: an OpenAI API key. Handed to Codex as `CODEX_API_KEY` |
| `{"method":"access_token"}` | `CODEX_ACCESS_TOKEN` | fixed | a JSON string: a personal access token or an Agent Identity JWT |
| `{"method":"auth_file"}` | `CODEX_AUTH` | **rotating** | a JSON object: Codex's whole `auth.json` (for example a ChatGPT login's tokens). Written before every loop, read back after, and written back to the vault slot, so a refreshed login survives |
| `{"method":"workload_identity","federation_rule_id":"…","context":"…"}` | `OPENAI_IDENTITY_TOKEN` | fixed | a JSON string: the identity token. `context` is optional |

Because `CODEX_AUTH` is rotating, its vault key gives one slot to one running container at a time. To
run several agents on one key, insert several values (several logins) into it; see `diverge-storage`.
A ChatGPT login speaks only to OpenAI's own backend, so do not combine `auth_file` with `provider`.

Minimal template body (API key):

```json
{
  "type": "agent",
  "image": { "digest": "<digest>", "references": [{ "registry": "docker.io", "name": "diverge-local/codex" }] },
  "memory": 2147483648,
  "disk": 4294967296,
  "arguments": { "model": "<model>", "auth": { "method": "api_key" } },
  "secrets": ["OPENAI_API_KEY"]
}
```

With a ChatGPT login instead: `"arguments":{"model":"<model>","auth":{"method":"auth_file"}}`,
`"secrets":["CODEX_AUTH"]`, and the vault value is the `auth.json` content as a JSON object.

## eliza (`diverge-agents/eliza`)

An elizaOS agent: a Node entry over `@elizaos/core`, with the plugins the arguments name. Its memory
is the agent's own database scope (reached through the proxy); its tools are the caller's MCP
servers; nothing is pre-wired but the database adapter.

`arguments`:

| member | required | meaning |
|---|---|---|
| `character` | yes | who the agent is. `name` is the one required field; optional `system` (the system prompt head), `bio` (lines), `topics`, `adjectives`, `style` (`{"all":[…],"chat":[…],"post":[…]}`), `message_examples` (groups of `{"name","text"}` turns) |
| `model_provider_plugins` | no | plugins that register **model handlers**, in priority order (first answers, later ones are failovers). **Absent means no model at all**, and the run fails as Eliza's own failure |
| `plugins` | no | every other plugin (actions, providers, services). A plugin that registers a model handler listed here is refused; one that registers none listed under `model_provider_plugins` is refused too |
| `memory` | no | switches, each off when absent: `advanced_capabilities`, `advanced_memory`, `relationships`, `documents`, `advanced_planning` |
| `generate_media` | no | whether the model may generate media; off when absent |
| `dependencies` | no | see above |

One plugin entry:

```json
{
  "package": "@elizaos/plugin-openai",
  "settings": { "OPENAI_BASE_URL": "https://api.openai.com/v1" },
  "secrets": ["OPENAI_API_KEY"]
}
```

- `package` — an npm spec, with or without a version. The image carries these pre-installed at its
  pin: `@elizaos/plugin-openai`, `plugin-embeddings`, `plugin-coding-tools`, `plugin-browser`,
  `plugin-documents`, `plugin-web-search`, `vault`. Any other package is installed at the run's start
  (so the container needs network), and the version resolved on the conversation's first run is the
  one every later run installs.
- `settings` — the plugin's own non-secret setting names, as JSON values.
- `secrets` — each either a bare string (a **fixed** secret, set as the setting of that name) or an
  object for a **rotating** one, saying where the plugin leaves the rotated value:

  ```json
  { "key": "GOOGLE_TOKENS", "rotates": true }
  { "key": "GOOGLE_TOKENS", "rotates": { "setting": "GOOGLE_TOKENS_NEW" } }
  { "key": "QWEN_OAUTH",    "rotates": { "file": ["root", ".qwen", "oauth_creds.json"] } }
  { "key": "WALLET",        "rotates": { "vault": "agent.{agent_id}.wallet.evm" } }
  ```

  `"rotates": false` is a fixed secret said the long way. A name declared both ways is rotating.

Secrets: **every name on any plugin entry's `secrets`**, each fixed or rotating as written. The
harness adds none of its own. The template must declare every one of those names.

Minimal template body (one OpenAI-compatible model provider):

```json
{
  "type": "agent",
  "image": { "digest": "<digest>", "references": [{ "registry": "docker.io", "name": "diverge-local/eliza" }] },
  "memory": 2147483648,
  "disk": 4294967296,
  "arguments": {
    "character": { "name": "Ada", "bio": ["A careful research assistant."] },
    "model_provider_plugins": [
      { "package": "@elizaos/plugin-openai", "secrets": ["OPENAI_API_KEY"] }
    ]
  },
  "secrets": ["OPENAI_API_KEY"]
}
```

Not available by design: connectors (discord, telegram, slack and the like), local inference,
sub-agent orchestration, scheduling. An agent that must act on such a service gets an MCP tool for it.

## cc (`diverge-agents/cc`)

A Claude Code agent: the harness installs the pinned Claude Code at container start (the container
needs network), runs it as a subprocess and serves the loop. Permissions are skipped inside it; the
container is the sandbox.

`arguments`:

| member | required | meaning |
|---|---|---|
| `model` | yes | the model to run |
| `tools` | no | the built-in tools Claude Code is given, one boolean switch each. **Absent or `false` is off**; the whole object absent is every switch off |
| `thinking` | no | whether to think before answering |
| `effort` | no | `"low"`, `"medium"`, `"high"`, `"xhigh"`, `"max"`; meaningful only with `thinking` on |
| `dependencies` | no | see above |

`tools` switches: `bash`, `read`, `write`, `edit`, `notebook_edit`, `glob`, `grep`, `task`,
`send_message`, `list_agents`, `task_output`, `task_stop`, `monitor`, `workflow`, `todo_write`,
`task_create`, `task_get`, `task_update`, `task_list`, `report_findings`, `repl`, `web_fetch`,
`web_search`, `enter_worktree`, `exit_worktree`, `sleep`.

Always on, with no switch: the caller's MCP tools (attached tools and dependencies), the MCP resource
built-ins, `Skill` (skills are directories mounted under `~/.claude/skills`) and `ToolSearch`. Always
off: anything that waits on a user or outlives the run (plan mode, questions, cron, artifacts).

Secrets: **none**. Claude Code logs in by itself; no argument names a credential, and a name the
template declares is one the program never asks for. How a login reaches the container is not
established (see the main file's "Not verified").

Minimal template body:

```json
{
  "type": "agent",
  "image": { "digest": "<digest>", "references": [{ "registry": "docker.io", "name": "diverge-local/cc" }] },
  "memory": 4294967296,
  "disk": 8589934592,
  "arguments": {
    "model": "<model>",
    "tools": { "bash": true, "read": true, "write": true, "edit": true, "glob": true, "grep": true }
  }
}
```
