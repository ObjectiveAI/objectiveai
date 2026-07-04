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
    // The viewer requires the daemon's ws:// connect URL. `run`'s
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

    // Derive the daemon WS auth signature from the effective secret
    // (`DAEMON_SECRET` env, else the on-disk `viewer.secret`):
    // `sha256=<hex(SHA256(secret))>` — the same one-way math as
    // `generate_viewer_secret_signature_pair`. Clients send it
    // verbatim in the first-message auth preamble (the SDK
    // `AuthEnvelope`) on every daemon WebSocket connection.
    let daemon_signature = ctx.daemon_secret().await?.as_deref().map(|secret| {
        use sha2::{Digest, Sha256};
        let hash = Sha256::digest(secret.as_bytes());
        format!(
            "sha256={}",
            hash.iter().map(|b| format!("{:02x}", b)).collect::<String>()
        )
    });

    // The child inherits the cli's environment; every env key the
    // viewer's config reads (`EnvConfigBuilder` in
    // `objectiveai-viewer/src-tauri/src/run.rs`: DAEMON_ADDRESS,
    // DAEMON_SIGNATURE, SUPPRESS_OUTPUT, OBJECTIVEAI_DIR,
    // OBJECTIVEAI_STATE) is set explicitly here when known.
    // DAEMON_ADDRESS is the daemon's full ws:// connect URL (always
    // set — required above; note it shadows any inherited value, which
    // in the cli's namespace would be a bind address, a different
    // semantic). DAEMON_SIGNATURE is derived from DAEMON_SECRET when
    // the cli has one; otherwise any inherited DAEMON_SIGNATURE from
    // the invoking shell is left as-is (the spawner may know the
    // signature without the secret).
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
