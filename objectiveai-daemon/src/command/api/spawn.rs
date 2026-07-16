//! Internal api-server spawn — the daemon starts `objectiveai-api` as
//! a leashed resident child when something needs it (there is no wire
//! `api spawn` command anymore; `Context::api_client()` is the entry).
//! The child announces its client-connect URL over the stdout ready
//! handshake; a live resident child short-circuits to its cached
//! address.

use crate::context::Context;
use crate::error::Error;

/// Every env key the api's config reads (`EnvConfigBuilder` in
/// `objectiveai-api/src/run.rs`) except `OBJECTIVEAI_DIR`, which the
/// spawn sets explicitly. The child inherits the cli's environment
/// (toolchain + machine identity — the dev `bin` entries are cargo-run
/// shims); scrubbing exactly the api's own config keys keeps the
/// spawning shell's settings from leaking into a server that other
/// processes will share. Deliberately scrubbed rather than forwarded:
/// OBJECTIVEAI_STATE (the api is global; it resolves its own state
/// default rather than inheriting whichever state the spawning cli
/// happened to run in) and ADDRESS/PORT (the api defaults to
/// 127.0.0.1 on an ephemeral port and publishes the bound URL in its
/// lock). The api's config then comes from `<OBJECTIVEAI_DIR>/.env`
/// (its main loads it non-overridingly) and its own defaults.
const API_CONFIG_ENV: &[&str] = &[
    "OBJECTIVEAI_ADDRESS",
    "OBJECTIVEAI_AUTHORIZATION",
    "OPENROUTER_ADDRESS",
    "OPENROUTER_AUTHORIZATION",
    "GITHUB_AUTHORIZATION",
    "MCP_AUTHORIZATION",
    "USER_AGENT",
    "HTTP_REFERER",
    "X_TITLE",
    "COMMIT_AUTHOR_NAME",
    "COMMIT_AUTHOR_EMAIL",
    "CLAUDE_AGENT_SDK_ENABLED",
    "CLAUDE_AGENT_SDK_RATE_LIMIT_MAX_RETRIES",
    "CLAUDE_AGENT_SDK_RATE_LIMIT_MAX_WAIT_SECS",
    "CLAUDE_AGENT_SDK_QUERY_LIMIT",
    "CODEX_SDK_ENABLED",
    "CODEX_SDK_RATE_LIMIT_MAX_RETRIES",
    "CODEX_SDK_RATE_LIMIT_MAX_WAIT_SECS",
    "CODEX_SDK_QUERY_LIMIT",
    "AGENT_COMPLETIONS_BACKOFF_MAX_ELAPSED_TIME",
    "MCP_BACKOFF_MAX_ELAPSED_TIME",
    "GITHUB_BACKOFF_MAX_ELAPSED_TIME",
    "AGENT_COMPLETIONS_FIRST_CHUNK_TIMEOUT",
    "MCP_CONNECT_TIMEOUT",
    "MCP_ENCRYPTION_KEY",
    "OBJECTIVEAI_STATE",
    "MOCK_DELAY_MS",
    "MOCK_MAX_TOOL_CALLS",
    "ADDRESS",
    "PORT",
];

/// The spawn flow itself (used by `Context::api_client()`).
/// Idempotent and cheap when the server is already up: a live
/// resident child returns its cached URL without spawning.
pub async fn spawn(ctx: &Context) -> Result<String, Error> {
    let bin = if cfg!(windows) {
        "objectiveai-api.exe"
    } else {
        "objectiveai-api"
    };
    let exe = ctx.filesystem.bin_dir().join(bin);

    // Project the configured api knobs onto the spawned api's env —
    // each ONLY when the user explicitly set it (the keys are scrubbed
    // above, so when unset the api resolves them itself from
    // `<OBJECTIVEAI_DIR>/.env` then its built-in default):
    //   • `api.mcp_connect_timeout_ms` → MCP_CONNECT_TIMEOUT (the
    //     initialize-handshake bound; the api forwards it to the proxy
    //     it hosts).
    //   • `api.backoff_max_elapsed_time_ms` → EVERY backoff max-elapsed
    //     env the api has (agent-completions, mcp, github).
    // NOTE: the CALL budget is deliberately NOT an env — it is the
    // per-request `X-MCP-CALL-TIMEOUT` header the daemon's OWN HTTP
    // client sends from `api.mcp_call_timeout_ms` (see
    // `context::build_http_client`).
    let connect_ms = ctx.resolve_mcp_connect_timeout_ms_opt().await?;
    let backoff_ms = ctx.resolve_backoff_max_elapsed_time_ms_opt().await?;

    let address = crate::spawn::spawn_leashed_until_ready(ctx, "api", &exe, |cmd| {
        for key in API_CONFIG_ENV {
            cmd.env_remove(key);
        }
        cmd.env("OBJECTIVEAI_DIR", ctx.filesystem.dir())
            .env("SUPPRESS_OUTPUT", "true");
        if let Some(connect_ms) = connect_ms {
            cmd.env("MCP_CONNECT_TIMEOUT", connect_ms.to_string());
        }
        if let Some(backoff_ms) = backoff_ms {
            let v = backoff_ms.to_string();
            cmd.env("AGENT_COMPLETIONS_BACKOFF_MAX_ELAPSED_TIME", &v)
                .env("MCP_BACKOFF_MAX_ELAPSED_TIME", &v)
                .env("GITHUB_BACKOFF_MAX_ELAPSED_TIME", &v);
        }
    })
    .await?;
    address.ok_or_else(|| {
        Error::Spawn(
            "objectiveai-api".to_string(),
            std::io::Error::other("api announced ready with no address"),
        )
    })
}
