//! `laboratories create` — launch the DETACHED `objectiveai-laboratory`
//! manager for this id. The manager owns everything from here: the
//! per-state id lock, the podman container, the MCP connection, and
//! the dial-out to the daemon's `/laboratory` route. Idempotent — a
//! held id lock means a manager is already running and the spec is
//! simply echoed back. Only client-side laboratories exist today.

use objectiveai_sdk::cli::command::laboratories::create::{Kind, Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    // Only `Client` exists today; the match stays exhaustive so adding
    // `Server` later forces a decision here.
    match request.kind {
        Kind::Client => {}
    }

    // The manager dials the daemon's /laboratory route — the daemon
    // must be up first (idempotent; returns the published ws:// URL).
    let daemon_address = crate::command::daemon::spawn::spawn(ctx).await?;
    let signature = ctx
        .config
        .daemon_secret
        .as_deref()
        .map(crate::websockets::daemon_auth::derive_signature);

    let exe = ctx.filesystem.bin_dir().join(if cfg!(windows) {
        "objectiveai-laboratory.exe"
    } else {
        "objectiveai-laboratory"
    });
    let lock_dir = ctx.filesystem.state_dir().join("locks").join("laboratories");
    let objectiveai_dir = ctx.filesystem.dir().clone();
    let state = ctx.filesystem.state().to_string();

    crate::spawn::spawn_until_lock_published(&exe, &lock_dir, &request.id, |cmd| {
        cmd.arg("--id")
            .arg(&request.id)
            .arg("--image")
            .arg(&request.image)
            .arg("--cwd")
            .arg(&request.cwd)
            .arg("--daemon-address")
            .arg(&daemon_address)
            .arg("--objectiveai-dir")
            .arg(&objectiveai_dir)
            .arg("--objectiveai-state")
            .arg(&state)
            .arg("--suppress-output");
        for mount in &request.mounts {
            cmd.arg("--mount")
                .arg(format!("{}:{}", mount.host, mount.container));
        }
        for env in &request.env {
            cmd.arg("--env").arg(format!("{}={}", env.key, env.value));
        }
        // The authorization signature travels by ENV VAR only (never
        // argv); cleared when the daemon is secretless so the child
        // can't inherit a stale one.
        match &signature {
            Some(s) => {
                cmd.env("DAEMON_SIGNATURE", s);
            }
            None => {
                cmd.env_remove("DAEMON_SIGNATURE");
            }
        }
    })
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
