//! `api spawn` — start the `objectiveai-api` server in the background,
//! using `api.address` + `api.port` from on-disk config.

use objectiveai_sdk::cli::command::api::spawn::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, _request: Request) -> Result<Response, Error> {
    let mut config = ctx.filesystem.read_config().await?;

    let address = config
        .api()
        .get_address()
        .ok_or(Error::MissingArgs(
            "api.address unset; run `objectiveai config api address set <addr>`",
        ))?
        .to_string();
    let port = config.api().get_port().ok_or(Error::MissingArgs(
        "api.port unset; run `objectiveai config api port set <port>`",
    ))?;

    crate::spawn::ensure_not_running("objectiveai-api")?;

    // Forward every set `ApiConfig` field to the spawned server as the
    // env var objectiveai-api reads (`objectiveai-api/src/run.rs`
    // ConfigBuilder). Unset fields are omitted so the server falls back
    // to its own env / defaults. Mapping rules:
    //   claude_agent_sdk          -> CLAUDE_AGENT_SDK_ENABLED ("true"/"false")
    //   codex_sdk                 -> CODEX_SDK_ENABLED ("true"/"false")
    //   objectiveai_authorization -> OBJECTIVEAI_AUTHORIZATION
    //   openrouter_authorization  -> OPENROUTER_AUTHORIZATION
    //   github_authorization      -> GITHUB_AUTHORIZATION
    //   mcp_authorization         -> MCP_AUTHORIZATION (JSON object string)
    //   user_agent                -> USER_AGENT
    //   http_referer              -> HTTP_REFERER
    //   x_title                   -> X_TITLE
    //   commit_author_name        -> COMMIT_AUTHOR_NAME
    //   commit_author_email       -> COMMIT_AUTHOR_EMAIL
    // (address/port travel as ADDRESS/PORT via the spawn helper itself.)
    let mut extra_env: Vec<(&str, String)> = Vec::new();
    {
        let api = config.api();
        if let Some(v) = api.get_claude_agent_sdk() {
            extra_env.push(("CLAUDE_AGENT_SDK_ENABLED", v.to_string()));
        }
        if let Some(v) = api.get_codex_sdk() {
            extra_env.push(("CODEX_SDK_ENABLED", v.to_string()));
        }
        if let Some(v) = api.get_objectiveai_authorization() {
            extra_env.push(("OBJECTIVEAI_AUTHORIZATION", v.to_string()));
        }
        if let Some(v) = api.get_openrouter_authorization() {
            extra_env.push(("OPENROUTER_AUTHORIZATION", v.to_string()));
        }
        if let Some(v) = api.get_github_authorization() {
            extra_env.push(("GITHUB_AUTHORIZATION", v.to_string()));
        }
        if let Some(v) = api.get_mcp_authorization() {
            if let Ok(json) = serde_json::to_string(v) {
                extra_env.push(("MCP_AUTHORIZATION", json));
            }
        }
        if let Some(v) = api.get_user_agent() {
            extra_env.push(("USER_AGENT", v.to_string()));
        }
        if let Some(v) = api.get_http_referer() {
            extra_env.push(("HTTP_REFERER", v.to_string()));
        }
        if let Some(v) = api.get_x_title() {
            extra_env.push(("X_TITLE", v.to_string()));
        }
        if let Some(v) = api.get_commit_author_name() {
            extra_env.push(("COMMIT_AUTHOR_NAME", v.to_string()));
        }
        if let Some(v) = api.get_commit_author_email() {
            extra_env.push(("COMMIT_AUTHOR_EMAIL", v.to_string()));
        }
    }

    let bin = if cfg!(windows) {
        "objectiveai-api.exe"
    } else {
        "objectiveai-api"
    };
    let exe = ctx.filesystem.base_dir().join("bin").join(bin);

    let listening =
        crate::spawn::spawn_and_wait_for_listening(&exe, &address, port, &extra_env).await?;
    Ok(Response { listening })
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
