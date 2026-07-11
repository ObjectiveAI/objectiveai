//! `laboratories connect` — spawn the DETACHED resident manager that
//! connects a created laboratory to a daemon. `address` unset = the
//! LOCAL daemon (ensured + resolved here); set = any remote daemon,
//! which is how a laboratory on this machine serves a daemon
//! elsewhere. One manager per `(id, address)`, enforced by the
//! manager's `connect_lock_key` lock — simultaneous connects to the
//! same pair resolve to exactly one manager, and both callers succeed
//! idempotently (the api/db/mcp spawn discipline).

use objectiveai_sdk::cli::command::laboratories::connect::{Request, Response};
use objectiveai_sdk::client_objectiveai_mcp::laboratory::connect_lock_key;

use crate::context::Context;
use crate::error::Error;

/// How long a LOCAL connect waits for the manager to appear in the
/// daemon registry. Generous: the container may be cold.
const CONNECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(600);

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    // Resolve the daemon: an explicit address is used verbatim
    // (remote); otherwise ensure the LOCAL daemon and use its
    // published ws:// URL.
    let (address, local) = match &request.address {
        Some(address) => (address.clone(), false),
        None => (crate::command::daemon::spawn::spawn(ctx).await?, true),
    };
    // The authorization signature: for the LOCAL daemon it derives
    // from this state's bare `SECRET`; for a REMOTE daemon the remote's
    // signature is the caller's to supply via the `DAEMON_SIGNATURE`
    // environment variable.
    let signature = if local {
        ctx.config
            .daemon_secret
            .as_deref()
            .map(crate::websockets::daemon_auth::derive_signature)
    } else {
        std::env::var("DAEMON_SIGNATURE").ok()
    };

    let exe = ctx.filesystem.bin_dir().join(if cfg!(windows) {
        "objectiveai-laboratory.exe"
    } else {
        "objectiveai-laboratory"
    });
    let lock_dir = ctx.filesystem.state_dir().join("locks").join("laboratories");
    let lock_key = connect_lock_key(&request.id, &address);
    let objectiveai_dir = ctx.filesystem.dir().clone();
    let state = ctx.filesystem.state().to_string();

    crate::spawn::spawn_until_lock_published(&exe, &lock_dir, &lock_key, |cmd| {
        cmd.arg("connect")
            .arg("--id")
            .arg(&request.id)
            .arg("--address")
            .arg(&address)
            .arg("--objectiveai-dir")
            .arg(&objectiveai_dir)
            .arg("--objectiveai-state")
            .arg(&state)
            .arg("--suppress-output");
        // The signature travels by ENV VAR only (never argv); cleared
        // when absent so the child can't inherit a stale one.
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

    // Readiness. LOCAL: connected = visible in the daemon registry —
    // poll it, failing fast if the manager dies (its lock releases).
    // REMOTE: this machine cannot see the remote registry; lock
    // publication is the whole contract (the manager retries its dial
    // forever).
    if local {
        let deadline = std::time::Instant::now() + CONNECT_TIMEOUT;
        loop {
            // In-process: poll the connected-laboratory registry directly
            // (was the laboratories.sock `List`).
            if let Some(hubs) = ctx.resident_hubs()
                && hubs.laboratories.list().iter().any(|l| l.id == request.id)
            {
                break;
            }
            if !objectiveai_sdk::lockfile::try_held(&lock_dir, &lock_key).await {
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
    }

    Ok(Response {
        id: request.id,
        address,
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::laboratories::connect as sdk;
    use objectiveai_sdk::cli::command::laboratories::connect::request_schema::{
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
    use objectiveai_sdk::cli::command::laboratories::connect as sdk;
    use objectiveai_sdk::cli::command::laboratories::connect::response_schema::{
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
