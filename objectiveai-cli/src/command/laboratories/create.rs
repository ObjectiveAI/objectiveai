//! `laboratories create` — create + start a laboratory container via podman,
//! injecting and running the bundled lab MCP binary. Echoes back what it
//! created. Only client-side laboratories are supported today.

use objectiveai_sdk::cli::command::laboratories::create::{Kind, Request, Response};

use crate::context::Context;
use crate::error::Error;
use objectiveai_sdk::podman::laboratory;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    // Only `Client` exists today; the match stays exhaustive so adding
    // `Server` later forces a decision here.
    match request.kind {
        Kind::Client => {}
    }

    let podman_mounts: Vec<laboratory::Mount> = request
        .mounts
        .iter()
        .map(|m| laboratory::Mount {
            host: m.host.clone(),
            container: m.container.clone(),
        })
        .collect();
    let env: Vec<(String, String)> = request
        .env
        .iter()
        .map(|e| (e.key.clone(), e.value.clone()))
        .collect();

    laboratory::create(
        ctx.podman_runtime(),
        ctx.filesystem.state(),
        &ctx.filesystem.bin_dir().join("objectiveai-mcp-laboratory"),
        &request.id,
        &request.image,
        &podman_mounts,
        &env,
        &request.cwd,
    )
    .await?;

    Ok(Response {
        id: request.id,
        image: request.image,
        mounts: request.mounts,
        env: request.env,
        cwd: request.cwd,
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::laboratories::create as sdk;
    use objectiveai_sdk::cli::command::laboratories::create::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::Request),
        ))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::laboratories::create as sdk;
    use objectiveai_sdk::cli::command::laboratories::create::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::Response),
        ))
    }
}
