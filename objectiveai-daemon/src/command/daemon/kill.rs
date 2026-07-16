//! `daemon kill` — stop the per-state plugin daemon.
//!
//! This handler runs INSIDE the daemon (every command reaches it via
//! `/execute`), so it must never kill itself: a self-`TerminateProcess`
//! would drop the /execute stream before the response could be sent. Killing
//! the daemon is the thin CLI's job — it resolves the daemon-lock owner
//! and signals it directly, off the wire. So a `daemon kill` that arrives
//! here (e.g. from a non-CLI `/execute` client) is rejected. The CLI's
//! `daemon kill` never reaches this handler.

use objectiveai_sdk::cli::command::daemon::kill::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
    Err(Error::CannotKillSelf)
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
