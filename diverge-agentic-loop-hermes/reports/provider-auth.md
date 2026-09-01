# Provider auth as arguments: the application table

From the pinned source (`sources/hermes-agent`, v0.20.6). This is
the record behind the SDK's `hermes/provider/` module: for every
bundled provider, the exact mechanism the harness binary must
perform to apply request-supplied credentials before starting
`hermes gateway`, and which providers cannot be expressed as
arguments at all.

## How a key actually gets found

Every api-key provider funnels through
`hermes_cli/auth.py:731 _resolve_api_key_provider_secret` →
`auth.py:7414 resolve_api_key_provider_credentials` →
`hermes_cli/runtime_provider.py:2407-2505`. Three deployment facts
govern everything below:

- **`~/.hermes/.env` SHADOWS the process environment.** Keys are
  read with `get_env_value_prefer_dotenv` (`hermes_cli/config.py:
  4666`, called at `auth.py:754`; pool seeding likewise via
  `agent/credential_pool.py:2938`). A stale `.env` in the image
  would silently override injected credentials. THE RULE: the
  harness sets canonical env vars in the GATEWAY PROCESS
  environment, and the image guarantees `~/.hermes/.env` does not
  exist. Process env is also the one mechanism that covers bedrock,
  whose botocore chain never reads Hermes's `.env`.
- Provider selection is `model.provider` in config.yaml (else the
  `HERMES_INFERENCE_PROVIDER` env, `runtime_provider.py:659`); the
  model is `model.default`. The harness owns config.yaml already.
- Endpoint overrides (`*_BASE_URL`) exist for most providers but
  are NOT part of our argument surface: a provider's fixed endpoint
  is its identity, and anything off-endpoint is what `custom` is
  for. The two exceptions (azure-foundry required, zai optional)
  are argued in place below.

## Key-only providers (32)

Application: set the canonical env var to the request's `api_key`.

| provider | canonical env var | notes |
|---|---|---|
| actual | `ACTUAL_API_KEY` | |
| ai-gateway | `AI_GATEWAY_API_KEY` | |
| alibaba | `DASHSCOPE_API_KEY` | |
| alibaba-coding-plan | `ALIBABA_CODING_PLAN_API_KEY` | falls back to `DASHSCOPE_API_KEY`; set the specific one |
| anthropic | `ANTHROPIC_API_KEY` | `CLAUDE_CODE_OAUTH_TOKEN` is listed but NOT usable as a key (`auth.py:398-407`) |
| arcee | `ARCEEAI_API_KEY` | not `ARCEE_API_KEY` |
| commandcode | `COMMANDCODE_API_KEY` | |
| commandcode-anthropic | `COMMANDCODE_API_KEY` | same key, Anthropic wire |
| deepinfra | `DEEPINFRA_API_KEY` | |
| deepseek | `DEEPSEEK_API_KEY` | |
| fireworks | `FIREWORKS_API_KEY` | |
| gemini | `GEMINI_API_KEY` | `GOOGLE_API_KEY` is checked FIRST — the harness must never set it |
| gmi | `GMI_API_KEY` | |
| huggingface | `HF_TOKEN` | |
| kilocode | `KILOCODE_API_KEY` | |
| kimi-coding | `KIMI_API_KEY` | an `sk-kimi-` key silently retargets to the coding endpoint (`auth.py:650-663`) — internal, fine |
| kimi-coding-cn | `KIMI_CN_API_KEY` | registry entry has no base-url env (`auth.py:335-341`) |
| meta-ai | `MODEL_API_KEY` | Meta's documented name; read at plugin import, so must be set before gateway start — it always is |
| minimax | `MINIMAX_API_KEY` | |
| minimax-cn | `MINIMAX_CN_API_KEY` | |
| nebius-token-factory | `NEBIUS_API_KEY` | |
| novita | `NOVITA_API_KEY` | |
| nvidia | `NVIDIA_API_KEY` | |
| ollama-cloud | `OLLAMA_API_KEY` | |
| opencode-go | `OPENCODE_GO_API_KEY` | |
| opencode-zen | `OPENCODE_ZEN_API_KEY` | |
| openrouter | `OPENROUTER_API_KEY` | bespoke resolution (`runtime_provider.py:1429`); keys not starting `sk-or-` are DISCARDED with a warning (`auth.py:700-703`) |
| router | `RAMP_ROUTER_API_KEY` | |
| stepfun | `STEPFUN_API_KEY` | |
| upstage | `UPSTAGE_API_KEY` | |
| xai | `XAI_API_KEY` | `xai-oauth` is a separate, OAuth-only registry entry, not a bundled profile |
| xiaomi | `XIAOMI_API_KEY` | |

## Special mechanisms (7)

- **opencode-free** — keyless by design (`auth.py:487-496`);
  short-circuited to an anonymous placeholder before any credential
  check (`runtime_provider.py:2020-2031`). Nothing to apply.
- **copilot** — set `COPILOT_GITHUB_TOKEN`; the ghu_→api-token
  exchange is automatic (`auth.py:735-747`, `:7446-7465`). Token
  must be `gho_`/`ghu_`/`github_pat_` — classic `ghp_` PATs are
  rejected (`copilot_auth.py:63-70`). Setting the env var also
  skips the `gh auth token` fallback (`copilot_auth.py:104-110`).
- **azure-foundry** — set `AZURE_FOUNDRY_API_KEY` AND
  `AZURE_FOUNDRY_BASE_URL`; a missing base URL is a hard AuthError
  (`runtime_provider.py:1559-1563`), so it is a REQUIRED argument.
  The Entra-ID mode (`model.auth_mode: entra_id`,
  `runtime_provider.py:1584-1618`) rides ambient Azure identity —
  not an argument, deliberately unsupported.
- **custom** — set `CUSTOM_BASE_URL` (required; trust gate at
  `runtime_provider.py:98-125`) and, when a key is supplied, write
  `model.api_key` in config.yaml. With no key, Hermes injects the
  literal `no-key-required` (`runtime_provider.py:1483`).
- **zai** — set `GLM_API_KEY`, and `GLM_BASE_URL` when the request
  supplies its optional base_url. Without a pinned base URL, first
  use fires a parallel live probe of 4 endpoints × up to 6 models
  and caches the winner into `~/.hermes/auth.json`
  (`auth.py:808-968`) — a network round-trip and a disk write at
  start, and the right endpoint is genuinely caller-dependent
  (global vs CN account), hence the optional argument.
- **bedrock** — no provider key; botocore chain priority is
  `AWS_BEARER_TOKEN_BEDROCK` → `AWS_ACCESS_KEY_ID` +
  `AWS_SECRET_ACCESS_KEY` (+ `AWS_SESSION_TOKEN`) → profiles/IMDS
  (`agent/bedrock_adapter.py:431-481`). Apply the request's auth as
  the corresponding real env vars; region → `AWS_REGION` (Hermes
  falls back to `us-east-1`, `bedrock_adapter.py:515-543`).
- **vertex** — write the request's service-account JSON to a file
  and set `VERTEX_CREDENTIALS_PATH` (preferred over
  `GOOGLE_APPLICATION_CREDENTIALS`; `agent/vertex_adapter.py:
  92-107`). Project defaults from the SA JSON, overridable via
  `VERTEX_PROJECT_ID`; region via `VERTEX_REGION` (default
  `global`). The base URL is computed, never configured.

## Rotating OAuth: resources, not drops (4)

`nous`, `openai-codex`, `minimax-oauth` and `qwen-oauth` are OAuth
with single-use rotating refresh tokens, and their advertised key
env vars are VESTIGIAL — `NOUS_API_KEY` has
`api_key_env_vars=()` in the registry (`auth.py:250-258`) and
`QWEN_API_KEY`'s only occurrence is profile metadata — so the
state document is the only credential there is. They ride the
vocabulary as RESOURCES: caller-provided state the run rotates,
surfaced back to the caller afterward. The full seeding and
rotation record is `oauth-resources.md`.

## Dropped from the vocabulary (1)

- **copilot-acp** — spawns the external `copilot` CLI over ACP;
  auth lives inside that CLI, not in Hermes (`auth.py:7495-7532`).
  (Keyed Copilot stays, as `copilot`.)
