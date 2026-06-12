//! `api spawn` — start the `objectiveai-api` server in the background.
//!
//! The api is machine-wide (one per `OBJECTIVEAI_DIR`): its lock lives
//! at `<dir>/bin/locks` key `api`, and the lock contents are the
//! server's client-connect URL. If the lock is already held the server
//! is already up and its published URL is returned as-is.

use objectiveai_sdk::cli::command::api::spawn::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, _request: Request) -> Result<Response, Error> {
    let bin = if cfg!(windows) {
        "objectiveai-api.exe"
    } else {
        "objectiveai-api"
    };
    let exe = ctx.filesystem.bin_dir().join(bin);
    let lock_dir = ctx.filesystem.bin_dir().join("locks");

    // The spawned server gets a fresh env carrying the layout root and
    // nothing else. Deliberately NOT OBJECTIVEAI_STATE — the api is
    // global; it resolves its own state default rather than inheriting
    // whichever state the spawning cli happened to run in. No
    // ADDRESS/PORT either: the api defaults to 127.0.0.1 on an
    // ephemeral port and publishes the bound URL in its lock.
    let listening = crate::spawn::spawn_until_lock_published(&exe, &lock_dir, "api", |cmd| {
        cmd.env("OBJECTIVEAI_DIR", ctx.filesystem.dir())
            .env("SUPPRESS_OUTPUT", "true");
    })
    .await?;
    Ok(Response { listening })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::api::spawn as sdk;
    use objectiveai_sdk::cli::command::api::spawn::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::api::spawn as sdk;
    use objectiveai_sdk::cli::command::api::spawn::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
