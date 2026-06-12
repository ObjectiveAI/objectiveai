//! `viewer spawn` — start the `objectiveai-viewer` Tauri shell in the
//! background.
//!
//! The viewer is per-state: its lock lives at
//! `<dir>/state/<state>/locks` key `viewer`, and the lock contents are
//! the server's client-connect URL. If the lock is already held the
//! viewer is already up and its published URL is returned as-is.

use objectiveai_sdk::cli::command::viewer::spawn::{Request, Response};

use crate::context::Context;
use crate::error::Error;

/// The spawn flow itself, callable in-process (used by
/// `Context::viewer_client()` as well as the `viewer spawn` command).
/// Idempotent and cheap when the viewer is already up: a try_read of
/// the lock returns the published URL without spawning.
pub async fn spawn(ctx: &Context) -> Result<String, Error> {
    let mut config = ctx
        .filesystem
        .read_config_view(objectiveai_sdk::cli::command::GetScope::Final)
        .await?;
    let secret = config.viewer().get_secret().map(String::from);

    let bin = if cfg!(windows) {
        "objectiveai-viewer.exe"
    } else {
        "objectiveai-viewer"
    };
    let exe = ctx.filesystem.bin_dir().join(bin);
    let lock_dir = ctx.filesystem.state_dir().join("locks");

    // Fresh env: layout coordinates + the configured secret (omitted
    // when unset so the viewer falls back to its own default). No
    // ADDRESS/PORT — the viewer defaults to 127.0.0.1 on an ephemeral
    // port and publishes the bound URL in its lock.
    crate::spawn::spawn_until_lock_published(&exe, &lock_dir, "viewer", |cmd| {
        cmd.env("OBJECTIVEAI_DIR", ctx.filesystem.dir())
            .env("OBJECTIVEAI_STATE", ctx.filesystem.state())
            .env("SUPPRESS_OUTPUT", "true");
        if let Some(secret) = secret {
            cmd.env("VIEWER_SECRET", secret);
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
