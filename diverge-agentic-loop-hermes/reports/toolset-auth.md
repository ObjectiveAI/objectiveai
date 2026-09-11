# Toolset auth: the per-tool table

From the pinned source (`sources/hermes-agent`, v0.20.6). The
record behind the SDK's `hermes/toolsets/` module: what credentials
each toolset's tools actually read, which are STATIC (a value, set
once) versus MUTABLE (state Hermes rewrites — a resource by the
provider rules), and the evidence for the drops.

Resolution machinery, shared by nearly everything:
`hermes_cli/config.py:4629 get_env_value` (process env /
`~/.hermes/.env`, scope-aware) and
`tools/tool_backend_helpers.py:158 resolve_provider_secret`
(config.yaml value → env → credential pool).

## The table

| toolset | credentials read | required? | verdict |
|---|---|---|---|
| web | `TAVILY_API_KEY`, `EXA_API_KEY`, `PARALLEL_API_KEY`, `KEENABLE_API_KEY`, `FIRECRAWL_API_KEY`, `BRAVE_SEARCH_API_KEY`, `SEARXNG_URL` (`tools/web_tools.py:603-614`) | optional — the keyless MCP ring works with zero credentials (`plugins/web/keyless_mcp.py:734`) | STATIC |
| browser | `BROWSER_CDP_URL`, `BROWSERBASE_API_KEY`+`BROWSERBASE_PROJECT_ID`; local headless chromium needs nothing (`tools_config.py:626-633`) | optional | STATIC |
| terminal | none locally; remote backends (ssh/daytona/vercel/modal) keyed (`tools/terminal_tool.py:3932-4043`) — irrelevant, the container IS the sandbox | no | none |
| file | none (`tools/file_tools.py:2644`) | no | none |
| code_execution | none of its own; called tools' creds apply transitively (`tools/code_execution_tool.py:356-375`) | no | none |
| vision / video | the RUN's inference provider — aux chain main provider → openrouter → nous → deepinfra (`agent/auxiliary_client.py:7584-7600`); the `OPENROUTER_API_KEY` row in `TOOLSET_ENV_REQUIREMENTS` is a presence marker, "never prompted or read" (`tools_config.py:752-758`) | rides the agent's provider | provider-coupled |
| image_gen | `FAL_KEY` built-in (`tools/image_generation_tool.py:1413-1426`); openai/krea/deepinfra/xai plugins beside it | one provider | STATIC (FAL) |
| video_gen | `FAL_KEY` / `DEEPINFRA_API_KEY` / xai (`tools/video_generation_tool.py:154-173`) | one provider | STATIC (FAL) |
| x_search | `XAI_API_KEY` or xAI OAuth — see below | one of the two | key STATIC / OAuth MUTABLE |
| tts | keyless default (edge; `tts_tool.py:3730-3735`); keyed: `ELEVENLABS_API_KEY` and friends (`tts_tool.py:650-664`) | optional | STATIC |
| todo | none (`tools/todo_tool.py:343-345`) | no | none |
| memory | none — see below | no | built-in store |
| context_engine | none; ships `"tools": []` (`toolsets.py:225-229`) | n/a | empty |
| session_search | none; local SQLite `state.db` FTS5 (`tools/session_search_tool.py:1136-1142`) | no | none |
| delegation | none; children inherit the parent's key (`tools/delegate_tool.py:1168-1170`, `:4464-4478`) | no | provider-coupled |
| homeassistant | `HASS_TOKEN` required, `HASS_URL` optional defaulting to `homeassistant.local` mDNS (`tools/homeassistant_tool.py:31-36`, gate `:346-347`) | token yes | STATIC (LLAT, never rotated) |
| yuanbao | none in the tools — see below | n/a | platform-gated |
| spotify | `HERMES_SPOTIFY_CLIENT_ID` + `providers.spotify` OAuth state — see below | both | client_id STATIC, state MUTABLE |

## spotify — the one per-tool rotating resource

- PKCE, no client secret (`auth.py:3101-3110`, grant `:3292-3298`).
  `client_id` is BYO: Hermes ships no app, hard-raising
  `spotify_client_id_missing` (`auth.py:3044-3048`); resolution
  cascade explicit → `HERMES_SPOTIFY_CLIENT_ID` →
  `SPOTIFY_CLIENT_ID` → the state document's own `client_id`
  (`auth.py:3028-3048`).
- Tokens live in `~/.hermes/auth.json` under `providers.spotify`,
  written with `set_active=False` so Spotify never hijacks the
  inference provider (`auth.py:3413,3431`). Document shape
  (`_spotify_token_payload_to_state`, `auth.py:3256-3276`):
  client_id, redirect_uri, accounts/api base urls, scope,
  granted_scope, token_type, access_token, refresh_token,
  obtained_at, expires_at, expires_in, `auth_type: "oauth_pkce"`.
- ROTATING AND REWRITTEN CONSTANTLY: the client refreshes on
  construction and on any 401 (`plugins/spotify/client.py:43`,
  `:83-91`), and it is constructed per tool call
  (`plugins/spotify/tools.py:27-28`) — every Spotify tool
  invocation may rewrite auth.json (refresh skew 120s,
  `auth.py:169`; refresh-token replacement with omit-on-refresh
  fallback, `auth.py:3266-3270`). Terminal refresh failure
  quarantines the entry (`auth.py:3416-3434`).

Verdict: `client_id` is a static argument; the state document is a
resource — precisely the provider-module rules.

## x_search — key path only

Two paths: `XAI_API_KEY` (via `resolve_provider_secret`,
`tools/xai_http.py`), and SuperGrok OAuth at
`providers["xai-oauth"].tokens` with SINGLE-USE rotating refresh
tokens ("the forced refresh below must consume a rotating refresh
token exactly once", `agent/credential_pool.py:2352-2353`; rotation
write-back `auth.py:5164-5171`, `#43589` note `:4837-4841`).

The SDK takes the key path ONLY, deliberately: dispatch itself
prefers the key (`prefer_api_key=True`,
`tools/x_search_tool.py:143`) because "x_search is API-metered; the
subscription OAuth bearer answers /v1/responses in a degraded
no-citation mode — #88040" (`x_search_tool.py:16-18`). The OAuth
path is a worse product for this tool, and its state is a shared
store other toolsets also touch — not worth a resource.

## memory — the drop's evidence

The `memory` toolset is Hermes's OWN cross-session store: two
Markdown files under `$HERMES_HOME/memories/` — `MEMORY.md` and
`USER.md` (`tools/memory_tool.py:63-66`, read `:247-248`) —
injected into the system prompt as a frozen snapshot at session
start (`memory_tool.py:11-14`). No credentials
(`memory_tool.py:1207-1213`). External providers (honcho,
supermemory, mem0, …) are config-selected plugins riding the same
toolset switch (`agent/memory_manager.py:87-112`).

Dropped: our cross-run memory is the CONTINUATION — a second,
container-local memory that evaporates with the container (or
worse, half-persists via some future mount) would fight it.

## The other drops

- **context_engine**: zero tools in stock Hermes; runtime injection
  from an engine plugin that does not ship (report 9). Nothing to
  enable.
- **yuanbao**: the tools read no credentials — they call the LIVE
  Yuanbao platform adapter (`tools/yuanbao_tools.py:28-34`, gate
  `:420-428`), which an api_server-only gateway never creates
  (report 8). The `app_secret` machinery signs a short-lived WS
  token held only in memory (`gateway/platforms/yuanbao.py:422`).

## Deliberately unsupported: the managed Nous gateway

web / image_gen / video_gen / tts / browser / terminal(modal) can
all route through Nous's managed tool gateway, whose token is the
ROTATING Nous OAuth state (`tools/managed_tool_gateway.py:174-195`
→ `auth.py:6097`, rewrite `:6216-6233`, shared store
`<root>/shared/nous_auth.json` `:5495`). Nous OAuth left the
provider vocabulary and stays out here; `TOOL_GATEWAY_USER_TOKEN`
(`managed_tool_gateway.py:77-95`) is a static escape hatch we may
admit later if asked for.

## Shared-env rule

`image_gen` and `video_gen` both apply `FAL_KEY`. Where two
toolsets name the same underlying variable, the values MUST agree —
a request that supplies both differently is contradicting itself,
and the harness may refuse it.
