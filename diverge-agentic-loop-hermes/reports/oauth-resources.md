# Rotating OAuth state as resources

From the pinned source (`sources/hermes-agent`, v0.20.6). The
provider-auth report initially dropped the four rotating-OAuth
providers; that was wrong-way-round. The POINT of auth-as-arguments
is that rotation becomes supportable: the caller hands the current
OAuth state in as a RESOURCE (`*_resource` fields on the provider
structures), the run rotates it, and the rotated state is surfaced
back so the
caller's next request carries current credentials. This report
records the exact documents, where the harness seeds them, and the
gates around them. (`copilot-acp` stays dropped — its auth lives in
a spawned external CLI, not in Hermes.)

## auth.json fundamentals

- Path: `$HERMES_HOME/auth.json` (`auth.py:1123-1124`; HERMES_HOME
  → `~/.hermes` default). Load `_load_auth_store` `auth.py:1356`,
  save `_save_auth_store` `auth.py:1425`.
- Top level: `{"version": 1, "providers": {...}, "active_provider":
  ..., "credential_pool": ..., "suppressed_sources": ...}`.
  `version` is never validated on read and rewritten to 1 on save
  (`auth.py:109,1438`). Read acceptance is only "dict with a dict
  `providers` or `credential_pool`" (`auth.py:1404-1407`);
  anything else silently degrades to an empty store.
- Permissions: 0o600 is applied on WRITE only, best-effort
  (`auth.py:1447-1476`); a 0644 harness-written file reads fine.
  Parent dir tightened to 0o700 on save (`auth.py:1437`).
- Malformed JSON → copied to `auth.json.corrupt`, store degrades to
  empty (`auth.py:1376-1402`). Write carefully.
- `PYTEST_CURRENT_TEST` in the gateway env + default HERMES_HOME →
  RuntimeError (`auth.py:1130-1141`). Never export it.

## Provider selection: never rely on `active_provider`

`resolve_requested_provider` (`runtime_provider.py:647-663`):
explicit arg → config.yaml `model.provider` → the
`HERMES_INFERENCE_PROVIDER` env → `auto`; only in `auto` does the
8-step chain reach auth.json's `active_provider` — at step 6,
BEHIND `OPENAI_API_KEY`/`OPENROUTER_API_KEY` and provider env keys
(`auth.py:2203-2216,2291-2295`). The harness sets
`model.provider`, which wins outright; a stray env key can never
hijack the run. Also: omit `suppressed_sources` entirely when
seeding — a cloned store where someone ran `hermes auth remove`
leaves the provider inert (`auth.py:1888-1895`).

## The four resources

### nous → `providers.nous` in auth.json

Entry as minted by device-code login (`auth.py:9168-9189`).
Required: `access_token` (`auth.py:6607-6609`) + `refresh_token`
(needed whenever the token isn't a fresh invoke-JWT,
`auth.py:6635-6644`); everything else defaults
(`auth.py:6476-6481`). `agent_key*` need not be seeded — derived
from the invoke JWT at runtime (`auth.py:2715-2761`).
`portal_base_url`/`inference_base_url` are HOST-ALLOWLISTED and
healed to production defaults if off-list (`auth.py:6322-6330,
6483-6488`). Rewrite-on-refresh: state mutated in place and
persisted at `auth.py:6711` (`_save_provider_state_to_source`,
`auth.py:6556`), final unconditional persist `auth.py:6735`; also
mirrored to `<root>/shared/nous_auth.json` (`auth.py:6578,5495`).

### openai-codex → `providers.openai-codex` in auth.json

Nested: `{"tokens": {"access_token", "refresh_token"},
"auth_mode": "chatgpt", "last_refresh": ...}`
(`_save_codex_tokens`, `auth.py:3962-3987`). Both token fields
hard-required with typed errors (`_read_codex_tokens`,
`auth.py:3831-3854`); no expires field — expiry read from the JWT
`exp` claim; base URL is env/default, never stored
(`auth.py:4256-4259`). Rewrite-on-refresh: `auth.py:4180`, plus a
mirror into matching credential-pool entries
(`auth.py:3928-3959`).

### minimax-oauth → `providers.minimax-oauth` in auth.json

Flat entry (`auth.py:8831-8845`). Must be COMPLETE:
`access_token` (`auth.py:9015-9021`), `refresh_token`
(`auth.py:8859-8863`), and — bracket-indexed, KeyError if absent —
`inference_base_url` (`auth.py:9034`), `portal_base_url`
(`auth.py:8872`), `client_id` (`auth.py:8880`). `expires_at`
should be a future ISO8601: a bad value parses to 0.0 and forces
refresh, and resolve ALWAYS attempts a refresh anyway
(`auth.py:9022-9023`). Rewrite-on-refresh: `auth.py:8922` →
`_minimax_save_auth_state` (`auth.py:8763-8768`); terminal refresh
failure QUARANTINES (wipes) the entry (`auth.py:8926ff,9025`).

### qwen-oauth → `~/.qwen/oauth_creds.json` (+ a marker)

The Qwen CLI's own file, hardcoded to `Path.home()` — it ignores
HERMES_HOME (`auth.py:2793-2794`). Document: `access_token`,
`refresh_token`, `token_type`, `resource_url`, `expiry_date` in
epoch MILLISECONDS (`auth.py:2854-2859`); missing/stale expiry →
refresh on first use. Rewrite-on-refresh: `auth.py:2930` →
`_save_qwen_cli_tokens` (`auth.py:2822-2851`), and the rewrite is
LOSSY — exactly five keys survive (`auth.py:2923-2929`), so
nothing extra in the document outlives the first rotation. The
harness must ALSO write the token-free selection marker
`providers.qwen-oauth` (+`base_url` only) into auth.json
(`_mark_qwen_oauth_active`, `auth.py:2934-2950`) — that marker is
the harness's own, not caller data.

## Portability

No machine/device binding anywhere in auth.py — the stores move
freely between machines; the only constraint is the server-side
single-use rotation itself, which is exactly what the resource
round-trip exists to honor.
