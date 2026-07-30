//! `laboratories kill` — GRACEFULLY terminate this daemon's resident
//! laboratory-host child. [`kill_resident_child`] recognizes the host
//! by its stdio channel and sends the acked
//! [`objectiveai_sdk::child_stdio::ChildStdioCommand::Shutdown`] over
//! its stdin, then waits (unbounded — no force path) for true exit:
//! the host stops every regular container it serves and evaporates
//! every ephemeral before exiting. Stdin EOF (the channel drop that
//! follows) remains the host's shutdown backstop. Idempotent: no
//! running host is a count of zero, not an error.

use objectiveai_sdk::cli::command::laboratories::kill::{Request, Response};

use crate::command::kill_helpers::kill_resident_child;
use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub async fn execute(global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
    let killed = kill_resident_child(global, "laboratories").await;
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
