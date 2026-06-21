//! `daemon kill` — stop the per-state plugin daemon. Its leashed
//! `daemon: true` plugins die with it.

use objectiveai_sdk::cli::command::daemon::kill::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, _request: Request) -> Result<Response, Error> {
    let lock_dir = ctx.filesystem.state_dir().join("locks");

    // Live owner PID(s) of the daemon lock → kill them. The daemon's
    // `daemon: true` plugins are leashed to it, so they die with it.
    let pids = objectiveai_sdk::lockfile::owners(&lock_dir, super::DAEMON_LOCK_KEY)
        .await
        .map_err(|e| Error::Lockfile {
            key: super::DAEMON_LOCK_KEY.to_string(),
            source: e,
        })?;
    let mut killed = 0usize;
    for pid in pids {
        killed += crate::spawn::kill_pid(pid);
    }

    Ok(Response { killed })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::daemon::kill as sdk;
    use objectiveai_sdk::cli::command::daemon::kill::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::daemon::kill as sdk;
    use objectiveai_sdk::cli::command::daemon::kill::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
