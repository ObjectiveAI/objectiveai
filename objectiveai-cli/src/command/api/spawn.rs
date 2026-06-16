//! `api spawn` — start the `objectiveai-api` server in the background.
//!
//! The api is machine-wide (one per `OBJECTIVEAI_DIR`): its lock lives
//! at `<dir>/bin/locks` key `api`, and the lock contents are the
//! server's client-connect URL. If the lock is already held the server
//! is already up and its published URL is returned as-is.

use objectiveai_sdk::cli::command::api::spawn::{Request, Response};

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
    "AGENT_COMPLETIONS_BACKOFF_CURRENT_INTERVAL",
    "AGENT_COMPLETIONS_BACKOFF_INITIAL_INTERVAL",
    "AGENT_COMPLETIONS_BACKOFF_RANDOMIZATION_FACTOR",
    "AGENT_COMPLETIONS_BACKOFF_MULTIPLIER",
    "AGENT_COMPLETIONS_BACKOFF_MAX_INTERVAL",
    "AGENT_COMPLETIONS_BACKOFF_MAX_ELAPSED_TIME",
    "MCP_BACKOFF_CURRENT_INTERVAL",
    "MCP_BACKOFF_INITIAL_INTERVAL",
    "MCP_BACKOFF_RANDOMIZATION_FACTOR",
    "MCP_BACKOFF_MULTIPLIER",
    "MCP_BACKOFF_MAX_INTERVAL",
    "MCP_BACKOFF_MAX_ELAPSED_TIME",
    "GITHUB_BACKOFF_CURRENT_INTERVAL",
    "GITHUB_BACKOFF_INITIAL_INTERVAL",
    "GITHUB_BACKOFF_RANDOMIZATION_FACTOR",
    "GITHUB_BACKOFF_MULTIPLIER",
    "GITHUB_BACKOFF_MAX_INTERVAL",
    "GITHUB_BACKOFF_MAX_ELAPSED_TIME",
    "AGENT_COMPLETIONS_FIRST_CHUNK_TIMEOUT",
    "AGENT_COMPLETIONS_OTHER_CHUNK_TIMEOUT",
    "MCP_CONNECT_TIMEOUT",
    "MCP_CALL_TIMEOUT",
    "REVERSE_CHANNEL_TIMEOUT",
    "MCP_ENCRYPTION_KEY",
    "OBJECTIVEAI_STATE",
    "PERSISTENT_CACHE_TRANSIENT_TTL_MS",
    "MOCK_DELAY_MS",
    "MOCK_MAX_TOOL_CALLS",
    "ADDRESS",
    "PORT",
];

/// The spawn flow itself, callable in-process (used by
/// `Context::api_client()` as well as the `api spawn` command).
/// Idempotent and cheap when the server is already up: a try_read of
/// the lock returns the published URL without spawning.
pub async fn spawn(ctx: &Context) -> Result<String, Error> {
    let bin = if cfg!(windows) {
        "objectiveai-api.exe"
    } else {
        "objectiveai-api"
    };
    let exe = ctx.filesystem.bin_dir().join(bin);
    let lock_dir = ctx.filesystem.bin_dir().join("locks");

    // Project the configured `api.mcp_backoff` blob (timeouts + backoff)
    // onto the spawned api's env — but ONLY when the user explicitly set
    // it. The keys are scrubbed above, so when `mcp_backoff` is unset we
    // leave them unset and the api resolves them itself
    // (`<OBJECTIVEAI_DIR>/.env`, then its built-in default). That's
    // identical to passing the canonical default in production, and it
    // preserves the test `.env`'s `MCP_BACKOFF_*=0` fast-fail on the
    // shared (machine-wide, first-spawn-wins) api — unconditionally
    // setting the default would clobber that `0`. See
    // `Context::resolve_mcp_backoff_opt`. When set, the api forwards
    // these to the proxy it spawns, so this drives every server-side MCP
    // connection too.
    let backoff = ctx.resolve_mcp_backoff_opt().await?;

    crate::spawn::spawn_until_lock_published(&exe, &lock_dir, "api", |cmd| {
        for key in API_CONFIG_ENV {
            cmd.env_remove(key);
        }
        cmd.env("OBJECTIVEAI_DIR", ctx.filesystem.dir())
            .env("SUPPRESS_OUTPUT", "true");
        if let Some(backoff) = backoff {
            cmd.env("MCP_CONNECT_TIMEOUT", backoff.connect_timeout_ms.to_string())
                .env(
                    "MCP_BACKOFF_CURRENT_INTERVAL",
                    backoff.current_interval_ms.to_string(),
                )
                .env(
                    "MCP_BACKOFF_INITIAL_INTERVAL",
                    backoff.initial_interval_ms.to_string(),
                )
                .env(
                    "MCP_BACKOFF_RANDOMIZATION_FACTOR",
                    backoff.randomization_factor.to_string(),
                )
                .env("MCP_BACKOFF_MULTIPLIER", backoff.multiplier.to_string())
                .env("MCP_BACKOFF_MAX_INTERVAL", backoff.max_interval_ms.to_string())
                .env(
                    "MCP_BACKOFF_MAX_ELAPSED_TIME",
                    backoff.max_elapsed_time_ms.to_string(),
                )
                .env("MCP_CALL_TIMEOUT", backoff.call_timeout_ms.to_string());
        }
    })
    .await
}

pub async fn execute(ctx: &Context, _request: Request) -> Result<Response, Error> {
    Ok(Response {
        listening: spawn(ctx).await?,
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::api::spawn as sdk;
    use objectiveai_sdk::cli::command::api::spawn::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::api::spawn as sdk;
    use objectiveai_sdk::cli::command::api::spawn::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
