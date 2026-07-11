//! `laboratories create` — create the laboratory container, WAITED to
//! completion: the manager binary's `create` subcommand does the
//! podman create + MCP-binary injection and exits. The container is
//! NOT started and nothing connects to any daemon — that is
//! `laboratories connect`'s job. Errors if the id already exists.
//! Only client-side laboratories are supported today.

use objectiveai_sdk::cli::command::laboratories::create::{Kind, Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    // Only `Client` exists today; the match stays exhaustive so adding
    // `Server` later forces a decision here.
    match request.kind {
        Kind::Client => {}
    }

    let exe = ctx.filesystem.bin_dir().join(if cfg!(windows) {
        "objectiveai-laboratory.exe"
    } else {
        "objectiveai-laboratory"
    });
    let mut cmd = tokio::process::Command::new(&exe);
    cmd.arg("create")
        .arg("--id")
        .arg(&request.id)
        .arg("--image")
        .arg(&request.image)
        .arg("--cwd")
        .arg(&request.cwd)
        .arg("--objectiveai-dir")
        .arg(ctx.filesystem.dir())
        .arg("--objectiveai-state")
        .arg(ctx.filesystem.state());
    for mount in &request.mounts {
        cmd.arg("--mount")
            .arg(format!("{}:{}", mount.host, mount.container));
    }
    for env in &request.env {
        cmd.arg("--env").arg(format!("{}={}", env.key, env.value));
    }
    let output = cmd
        .output()
        .await
        .map_err(|e| Error::Laboratory(format!("spawn objectiveai-laboratory create: {e}")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Laboratory(format!(
            "objectiveai-laboratory create: {}",
            stderr.trim()
        )));
    }

    // Tell a running daemon its local set changed (best-effort).
    super::signal_local_changed(ctx).await;

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
    use objectiveai_sdk::cli::command::laboratories::create::request_schema::{
        Request, Response,
    };

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
    use objectiveai_sdk::cli::command::laboratories::create::response_schema::{
        Request, Response,
    };

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::Response),
        ))
    }
}
