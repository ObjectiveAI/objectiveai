# hermes (`diverge-agentic-loop-hermes`)

A Hermes agent (Nous Research's agent harness): `hermes gateway` laid down from the arguments and
driven over its API. Hermes is multi-provider; the `provider` argument picks the inference source and
**selects the secrets** its auth needs. The toolsets select more. No credential is ever an argument.

## `arguments`

| member | required | meaning |
|---|---|---|
| `provider` | yes | the inference source: an object whose `provider` member is the provider id, plus that provider's endpoint facts (table below) |
| `model` | yes | the model, in the provider's own naming |
| `system_prompt` | no | applied per request |
| `effort` | no | `"none"`, `"minimal"`, `"low"`, `"medium"`, `"high"`, `"xhigh"`, `"max"`, `"ultra"`. Absent is the provider's default; `"none"` is reasoning explicitly disabled |
| `toolsets` | no | Hermes's local capabilities, one switch each (below). Absent is everything off |
| `dependencies` | no | dependency tool templates; see [agent-loop-images.md](agent-loop-images.md) |

There is no temperature and no thinking-token budget. Skills are not an argument: they arrive as
mounts under `/root/.hermes/external-skills/`, one directory per skill holding its `SKILL.md`.

## Secrets

The program computes its declaration from the arguments: the provider's secrets plus each enabled
toolset's. **The template's `secrets` must list every name selected**, and the agent's create must
bind each to a vault key. A name selected twice (for example `XAI_API_KEY` by the `xai` provider and
by `x_search`) is one secret.

Fixed secrets are JSON strings unless noted. Rotating secrets are JSON objects (OAuth state) that the
harness writes where Hermes reads them, reads back after the gateway exits, and hands back to be
written into the vault slot they were locked from. A rotating key gives one value to one running
container at a time; see `diverge-storage`.

## Providers

`provider` is `{"provider":"<id>", …}`. Most take nothing but the id.

| `provider` id | other members | secrets (mode) |
|---|---|---|
| `actual` | | `ACTUAL_API_KEY` (fixed) |
| `ai-gateway` | | `AI_GATEWAY_API_KEY` (fixed) |
| `alibaba` | | `DASHSCOPE_API_KEY` (fixed) |
| `alibaba-coding-plan` | | `ALIBABA_CODING_PLAN_API_KEY` (fixed) |
| `anthropic` | | `ANTHROPIC_API_KEY` (fixed) |
| `arcee` | | `ARCEEAI_API_KEY` (fixed) |
| `azure-foundry` | `base_url` (required) | `AZURE_FOUNDRY_API_KEY` (fixed) |
| `bedrock` | `auth` (required): `"bearer-token"`, `"access-key"` or `"session-token"`; `region` (optional) | `bearer-token`: `AWS_BEARER_TOKEN_BEDROCK`; `access-key`: `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`; `session-token`: those two and `AWS_SESSION_TOKEN` (all fixed) |
| `commandcode` | | `COMMANDCODE_API_KEY` (fixed) |
| `commandcode-anthropic` | | `COMMANDCODE_API_KEY` (fixed) |
| `copilot` | | `COPILOT_GITHUB_TOKEN` (fixed): an OAuth or fine-grained token (`gho_`, `ghu_`, `github_pat_`); classic `ghp_` tokens are rejected by Hermes |
| `custom` | `base_url` (required): any OpenAI-compatible endpoint; `api_key` (optional boolean) | `CUSTOM_API_KEY` (fixed) only when `api_key` is `true`; none otherwise |
| `deepinfra` | | `DEEPINFRA_API_KEY` (fixed) |
| `deepseek` | | `DEEPSEEK_API_KEY` (fixed) |
| `fireworks` | | `FIREWORKS_API_KEY` (fixed) |
| `gemini` | | `GEMINI_API_KEY` (fixed) |
| `gmi` | | `GMI_API_KEY` (fixed) |
| `huggingface` | | `HF_TOKEN` (fixed) |
| `kilocode` | | `KILOCODE_API_KEY` (fixed) |
| `kimi-coding` | | `KIMI_API_KEY` (fixed) |
| `kimi-coding-cn` | | `KIMI_CN_API_KEY` (fixed) |
| `meta-ai` | | `MODEL_API_KEY` (fixed) |
| `minimax` | | `MINIMAX_API_KEY` (fixed) |
| `minimax-cn` | | `MINIMAX_CN_API_KEY` (fixed) |
| `minimax-oauth` | | `MINIMAX_OAUTH` (**rotating**, JSON object) |
| `nebius-token-factory` | | `NEBIUS_API_KEY` (fixed) |
| `nous` | | `NOUS_OAUTH` (**rotating**, JSON object) |
| `novita` | | `NOVITA_API_KEY` (fixed) |
| `nvidia` | | `NVIDIA_API_KEY` (fixed) |
| `ollama-cloud` | | `OLLAMA_API_KEY` (fixed) |
| `openai-codex` | | `OPENAI_CODEX_OAUTH` (**rotating**, JSON object) |
| `opencode-free` | | none (keyless) |
| `opencode-go` | | `OPENCODE_GO_API_KEY` (fixed) |
| `opencode-zen` | | `OPENCODE_ZEN_API_KEY` (fixed) |
| `openrouter` | | `OPENROUTER_API_KEY` (fixed); Hermes discards keys that do not start with `sk-or-` |
| `qwen-oauth` | | `QWEN_OAUTH` (**rotating**, JSON object: the CLI's token file) |
| `router` | | `RAMP_ROUTER_API_KEY` (fixed) |
| `stepfun` | | `STEPFUN_API_KEY` (fixed) |
| `upstage` | | `UPSTAGE_API_KEY` (fixed) |
| `vertex` | `project_id`, `region` (both optional) | `VERTEX_SERVICE_ACCOUNT_JSON` (fixed): the service-account key document **as a JSON string holding the document's text**, not as an object |
| `xai` | | `XAI_API_KEY` (fixed) |
| `xiaomi` | | `XIAOMI_API_KEY` (fixed) |
| `zai` | `base_url` (optional) | `GLM_API_KEY` (fixed) |

The provider ids are the kebab-case spellings above; confirm the exact member names for a provider
with the image's schema (`diverge run images info --jq '.info.schema'`), whose descriptions carry a
`SECRET:` paragraph per structure.

## Toolsets

`toolsets` is an object; every member absent is off. A plain switch takes `true`. A structured
toolset is on when its object is present (the empty object included).

| member | value | secrets (mode) |
|---|---|---|
| `terminal` | `true` | none |
| `file` | `true` | none |
| `code_execution` | `true` | none |
| `vision` | `true` | none (rides the run's inference provider) |
| `video` | `true` | none |
| `todo` | `true` | none |
| `session_search` | `true` | none |
| `memory` | `true` | none (Hermes's own `MEMORY.md` / `USER.md`) |
| `image_gen` | `true` | `FAL_KEY` (fixed) |
| `video_gen` | `true` | `FAL_KEY` (fixed; the same secret) |
| `x_search` | `true` | `XAI_API_KEY` (fixed) |
| `web` | `{}` for the keyless ring; `{"search": "tavily" \| "exa" \| "parallel" \| "keenable" \| "brave-free"}`; `{"search": {"searxng_url": "…"}}`; `{"extract": "firecrawl"}`; or both slots | search: `TAVILY_API_KEY`, `EXA_API_KEY`, `PARALLEL_API_KEY`, `KEENABLE_API_KEY` or `BRAVE_SEARCH_API_KEY`; none for SearXNG. extract: `FIRECRAWL_API_KEY`. All fixed |
| `browser` | `{}` for the container's own headless Chromium; `{"remote": "browserbase"}`; `{"remote": {"cdp_url": "…"}}` | Browserbase: `BROWSERBASE_API_KEY` and `BROWSERBASE_PROJECT_ID` (fixed); none otherwise |
| `tts` | `{}` for the free voice; `{"elevenlabs": true}` | `ELEVENLABS_API_KEY` (fixed) when `elevenlabs` is `true` |
| `homeassistant` | `{"url": "…"}` (required) | `HASS_TOKEN` (fixed) |
| `spotify` | `{"client_id": "…"}` (required) | `SPOTIFY_OAUTH` (**rotating**, JSON object: the login document) |

The caller's MCP tools (attached tools and dependencies) are always wired in and are not switches.
Sub-agent delegation, cron, clarify and the Discord toolsets are off by design and cannot be turned
on. Enabling a toolset without what it needs is not a registration error; the tool fails as itself
when used.

## Minimal template body

```json
{
  "type": "agent",
  "image": { "digest": "<digest>", "references": [{ "registry": "docker.io", "name": "diverge-local/hermes" }] },
  "memory": 4294967296,
  "disk": 8589934592,
  "arguments": {
    "provider": { "provider": "openrouter" },
    "model": "<vendor/model>",
    "toolsets": { "terminal": true, "file": true }
  },
  "secrets": ["OPENROUTER_API_KEY"]
}
```

```bash
diverge run vault insert '{"key":"or-key","value":"sk-or-…"}'
diverge run agents create '{"template":"<id>","name":"hermes-1","secrets":{"OPENROUTER_API_KEY":"or-key"}}'
```

A fuller one, with a keyed search backend and a local OpenAI-compatible endpoint that needs no key:

```json
"arguments": {
  "provider": { "provider": "custom", "base_url": "http://10.0.0.5:11434/v1" },
  "model": "<model>",
  "effort": "medium",
  "toolsets": { "web": { "search": "tavily" }, "terminal": true, "file": true, "memory": true }
},
"secrets": ["TAVILY_API_KEY"]
```
