//! `db kill` — stop the postmaster started by `db spawn`.
//! Idempotent: a count of zero is not an error.
//!
//! The target pid comes from `<state_dir>/db/postmaster.pid` line 1 —
//! killing by process name is NOT an option here, because the
//! `objectiveai-db` vehicle exits right after launching, and matching
//! on "postgres" would take out unrelated PostgreSQL servers on the
//! machine. A missing/stale pid file (or a pid that is no longer a
//! live process) yields `{killed: 0}`.

use objectiveai_sdk::cli::command::db::kill::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, _request: Request) -> Result<Response, Error> {
    let pid_file = ctx.filesystem.state_dir().join("db").join("postmaster.pid");
    let Ok(content) = tokio::fs::read_to_string(&pid_file).await else {
        return Ok(Response { killed: 0 });
    };
    let Some(pid) = content.lines().next().and_then(|l| l.trim().parse::<u32>().ok())
    else {
        return Ok(Response { killed: 0 });
    };
    let killed = crate::spawn::kill_pid(pid);
    Ok(Response { killed })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::db::kill as sdk;
    use objectiveai_sdk::cli::command::db::kill::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::db::kill as sdk;
    use objectiveai_sdk::cli::command::db::kill::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
