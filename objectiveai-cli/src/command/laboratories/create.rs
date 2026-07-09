//! `laboratories create` — launch the DETACHED `objectiveai-laboratory`
//! manager for this id. The manager owns everything from here: the
//! per-state id lock, the podman container, the MCP connection, and
//! the dial-out to the daemon's `/laboratory` route. Idempotent — a
//! held id lock means a manager is already running and the spec is
//! simply echoed back. Only client-side laboratories exist today.

use objectiveai_sdk::cli::command::laboratories::create::{Kind, Request, Response};
use objectiveai_sdk::client_objectiveai_mcp::laboratory::{SocketRequest, SocketResponse};

use crate::context::Context;
use crate::error::Error;

/// How long `create` waits for the spawned manager to CONNECT to the
/// daemon (appear in `laboratories list`). Generous on purpose: a
/// first-ever create may pull the container image, and the old
/// CLI-side create blocked through that too.
const CONNECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(600);

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
        cmd.arg("run")
            .arg("--id")
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

    // Readiness = CONNECTED: the manager acquires its id lock at
    // startup, but the laboratory only exists (for `list`, for the
    // conduit, for agents) once it has dialed /laboratory and
    // identified. Poll the registry until it appears — and fail fast
    // if the manager dies (its id lock releases) instead of spinning
    // out the full timeout.
    let deadline = std::time::Instant::now() + CONNECT_TIMEOUT;
    let state_dir = ctx.filesystem.state_dir();
    loop {
        match crate::websockets::websocket_laboratory::call_laboratories_socket(
            &state_dir,
            &SocketRequest::List,
        )
        .await
        {
            Ok(SocketResponse::List { laboratories })
                if laboratories.iter().any(|l| l.id == request.id) =>
            {
                break;
            }
            _ => {}
        }
        if !objectiveai_sdk::lockfile::try_held(&lock_dir, &request.id).await {
            return Err(Error::Laboratory(format!(
                "laboratory manager for '{}' exited before connecting to the daemon",
                request.id
            )));
        }
        if std::time::Instant::now() >= deadline {
            return Err(Error::Laboratory(format!(
                "laboratory '{}' did not connect to the daemon within {}s",
                request.id,
                CONNECT_TIMEOUT.as_secs()
            )));
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }

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
