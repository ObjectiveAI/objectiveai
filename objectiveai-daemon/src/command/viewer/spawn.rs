//! `viewer spawn` — start the `objectiveai-viewer` Tauri shell in the
//! background.
//!
//! The viewer is per-state: its lock lives at
//! `<dir>/state/<state>/locks` key `viewer`. The viewer is a WebSocket
//! CLIENT of the daemon's broadcast (not a server), so the lock content
//! is a plain readiness marker, not a URL. If the lock is already held
//! the viewer is already up.

use objectiveai_sdk::cli::command::viewer::spawn::{Request, Response};

use crate::context::Context;
use crate::error::Error;

/// The spawn flow itself. Idempotent and cheap when the viewer is
/// already up: a try_read of the lock returns without spawning.
pub async fn spawn(ctx: &Context) -> Result<String, Error> {
    // The viewer requires the daemon's http:// connect URL. `run`'s
    // producer tee ensured the daemon and recorded the address on the
    // ctx before this handler ran; if it's absent the daemon couldn't
    // be spawned, and a viewer without a daemon is useless — error out.
    let daemon_address = ctx
        .daemon_address()
        .ok_or(Error::DaemonAddressUnavailable)?
        .to_string();

    let bin = if cfg!(windows) {
        "objectiveai-viewer.exe"
    } else {
        "objectiveai-viewer"
    };
    let exe = ctx.filesystem.bin_dir().join(bin);
    let lock_dir = ctx.filesystem.state_dir().join("locks");

    // The daemon WS auth signature: the daemon's own bare `SIGNATURE`
    // env when set, else derived one-way from its bare `SECRET`
    // (`sha256=<hex(SHA256(secret))>`, the same math as
    // `generate_viewer_secret_signature_pair`). Clients send it
    // verbatim in the first-message auth preamble (the SDK
    // `AuthEnvelope`) on every daemon WebSocket connection.
    let daemon_signature = ctx.config.client_signature();

    // The child inherits the cli's environment; every env key the
    // viewer's config reads (`EnvConfigBuilder` in
    // `objectiveai-viewer/src-tauri/src/run.rs`: DAEMON_ADDRESS,
    // DAEMON_SIGNATURE, SUPPRESS_OUTPUT, OBJECTIVEAI_DIR,
    // OBJECTIVEAI_STATE) is set explicitly here when known.
    // `DAEMON_ADDRESS` is the daemon's full http:// connect URL the viewer
    // (a client) dials (always set — required above). `DAEMON_SIGNATURE`
    // is derived here from the daemon's own bare `SECRET` when it has one;
    // otherwise any inherited `DAEMON_SIGNATURE` is left as-is (the spawner
    // may know the signature without the secret). The daemon's own bind
    // config lives in the bare `ADDRESS`/`PORT`/`SECRET` namespace,
    // distinct from these client-facing `DAEMON_` vars.
    crate::spawn::spawn_until_lock_published(&exe, &lock_dir, "viewer", |cmd| {
        cmd.env("OBJECTIVEAI_DIR", ctx.filesystem.dir())
            .env("OBJECTIVEAI_STATE", ctx.filesystem.state())
            .env("SUPPRESS_OUTPUT", "true")
            .env("DAEMON_ADDRESS", &daemon_address);
        if let Some(signature) = daemon_signature {
            cmd.env("DAEMON_SIGNATURE", signature);
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
    use objectiveai_sdk::cli::command::viewer::spawn as sdk;
    use objectiveai_sdk::cli::command::viewer::spawn::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::viewer::spawn as sdk;
    use objectiveai_sdk::cli::command::viewer::spawn::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
