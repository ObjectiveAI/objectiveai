//! `laboratories delete` — remove the laboratory container, WAITED to
//! completion: the manager binary's `delete` subcommand does the
//! podman `rm -f` (force-removes even a running container, reclaiming
//! disk) and exits. A missing container is not an error. Signals a
//! running daemon afterward so the `/laboratories/*` streams update.
//! Only client-side laboratories are supported today.

use objectiveai_sdk::cli::command::laboratories::delete::{Kind, Request, Response};

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
    let output = tokio::process::Command::new(&exe)
        .arg("delete")
        .arg("--id")
        .arg(&request.id)
        .arg("--objectiveai-dir")
        .arg(ctx.filesystem.dir())
        .arg("--objectiveai-state")
        .arg(ctx.filesystem.state())
        .output()
        .await
        .map_err(|e| Error::Laboratory(format!("spawn objectiveai-laboratory delete: {e}")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Laboratory(format!(
            "objectiveai-laboratory delete: {}",
            stderr.trim()
        )));
    }

    // Tell a running daemon its local set changed (best-effort).
    super::signal_local_changed(ctx).await;

    Ok(Response { id: request.id })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::laboratories::delete as sdk;
    use objectiveai_sdk::cli::command::laboratories::delete::request_schema::{
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
    use objectiveai_sdk::cli::command::laboratories::delete as sdk;
    use objectiveai_sdk::cli::command::laboratories::delete::response_schema::{
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
