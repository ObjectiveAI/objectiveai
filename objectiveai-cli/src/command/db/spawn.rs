//! `db spawn` — start the `objectiveai-db` postgres supervisor in the
//! background.
//!
//! The supervisor is per-state: its lock lives at
//! `<dir>/state/<state>/locks` key `db`, and the lock contents are the
//! cluster's `postgresql://...` connection URL (the postmaster binds
//! 127.0.0.1 on a random free port). If the lock is already held the
//! cluster is already up and its published URL is returned as-is.

use objectiveai_sdk::cli::command::db::spawn::{Request, Response};

use crate::context::Context;
use crate::error::Error;
use crate::filesystem::config::DB_DEFAULT_PASSWORD;

pub async fn execute(ctx: &Context, _request: Request) -> Result<Response, Error> {
    let mut config = ctx
        .filesystem
        .read_config_view(objectiveai_sdk::cli::command::GetScope::Final)
        .await?;
    let password = config
        .db()
        .get_password()
        .unwrap_or(DB_DEFAULT_PASSWORD)
        .to_string();

    let bin = if cfg!(windows) {
        "objectiveai-db.exe"
    } else {
        "objectiveai-db"
    };
    let exe = ctx.filesystem.bin_dir().join(bin);
    let lock_dir = ctx.filesystem.state_dir().join("locks");

    // objectiveai-db is clap-args-only (no env): the layout
    // coordinates tell it to provision THIS cli's tree — postgres
    // binaries into the shared <dir>/bin/pg-bin, the cluster into
    // <dir>/state/<state>/db.
    let listening = crate::spawn::spawn_until_lock_published(&exe, &lock_dir, "db", |cmd| {
        cmd.arg("--objectiveai-dir")
            .arg(ctx.filesystem.dir())
            .arg("--objectiveai-state")
            .arg(ctx.filesystem.state())
            .arg("--pg-password")
            .arg(password);
    })
    .await?;
    Ok(Response { listening })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::db::spawn as sdk;
    use objectiveai_sdk::cli::command::db::spawn::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::db::spawn as sdk;
    use objectiveai_sdk::cli::command::db::spawn::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
