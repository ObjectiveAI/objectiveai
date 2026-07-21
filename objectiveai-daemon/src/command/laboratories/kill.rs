//! `laboratories kill` — GRACEFULLY terminate this daemon's resident
//! laboratory-host child. Closing the host's stdin (which
//! [`graceful_kill_resident_child`] does by taking the child off the
//! map) is its shutdown signal: it stops every regular container it
//! serves and evaporates every ephemeral before exiting. Idempotent:
//! no running host is a count of zero, not an error.

use objectiveai_sdk::cli::command::laboratories::kill::{Request, Response};

use crate::command::kill_helpers::graceful_kill_resident_child;
use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub async fn execute(global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
    let killed = graceful_kill_resident_child(global, "laboratories").await?;
    Ok(Response { killed })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::laboratories::kill as sdk;
    use objectiveai_sdk::cli::command::laboratories::kill::request_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::laboratories::kill as sdk;
    use objectiveai_sdk::cli::command::laboratories::kill::response_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
