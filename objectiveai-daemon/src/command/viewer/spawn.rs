//! `viewer spawn` — start the `objectiveai-viewer` Tauri shell in the
//! background.
//!
//! The viewer is per-state: its lock lives at
//! `<dir>/state/<state>/locks` key `viewer`. The viewer is an SSE
//! CLIENT of the daemon's broadcast (not a server), so the lock content
//! is a plain readiness marker, not a URL. If the lock is already held
//! the viewer is already up.

use objectiveai_sdk::cli::command::viewer::spawn::{Request, Response};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

/// The spawn flow itself. Idempotent and cheap when the viewer is
/// already up: a try_read of the lock returns without spawning.
pub async fn spawn(global: &GlobalContext, scoped: &ScopedContext) -> Result<String, Error> {
    // The viewer requires the daemon's http:// connect URL. `run`'s
    // producer tee ensured the daemon and recorded the address on the
    // global context before this handler ran; if it's absent the daemon couldn't
    // be spawned, and a viewer without a daemon is useless — error out.
    let daemon_address = global
        .daemon_address()
        .ok_or(Error::DaemonAddressUnavailable)?
        .to_string();

    let bin = if cfg!(windows) {
        "objectiveai-viewer.exe"
    } else {
        "objectiveai-viewer"
    };
    let exe = scoped.filesystem.bin_dir().join(bin);

    // The daemon auth signature: the daemon's own bare `SIGNATURE`
    // env when set, else derived one-way from its bare `SECRET`
    // (`sha256=<hex(SHA256(secret))>`, the same math as
    // `generate_viewer_secret_signature_pair`). Clients send it
    // verbatim in the `X-OBJECTIVEAI-SIGNATURE` header on every daemon
    // HTTP request (the `/laboratory` WebSocket keeps the `AuthEnvelope` preamble).
    let daemon_signature = global.client_signature();

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
    let _ = crate::spawn::spawn_leashed_until_ready(global, "viewer", &exe, |cmd| {
        // The viewer is a WINDOWED child (the release binary is
        // GUI-subsystem, so CREATE_NO_WINDOW never hides its window,
        // only a console-subsystem dev build's console). It is leashed
        // like every other resident child: the viewer dies with the
        // daemon BY DESIGN now.
        cmd.env("OBJECTIVEAI_DIR", scoped.filesystem.dir())
            .env("OBJECTIVEAI_STATE", scoped.filesystem.state())
            .env("SUPPRESS_OUTPUT", "true")
            .env("DAEMON_ADDRESS", &daemon_address);
        if let Some(signature) = daemon_signature {
            cmd.env("DAEMON_SIGNATURE", signature);
        }
    })
    .await?;
    Ok("ready".to_string())
}

pub async fn execute(global: &GlobalContext, scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
    Ok(Response {
        listening: spawn(global, scoped).await?,
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::viewer::spawn as sdk;
    use objectiveai_sdk::cli::command::viewer::spawn::request_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::viewer::spawn as sdk;
    use objectiveai_sdk::cli::command::viewer::spawn::response_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
