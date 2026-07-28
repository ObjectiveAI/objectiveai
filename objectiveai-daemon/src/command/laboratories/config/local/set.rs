//! `laboratories config local set` — write `laboratories.local` (the
//! desired state), then CONVERGE a running host to it
//! ([`crate::command::laboratories::spawn::converge`] — ack-gated;
//! the converge resolves the local daemon's coordinates exactly as
//! the host spawn does, so flipping `local` adds or drops that
//! connection). No live host ⇒ the converge no-ops immediately (no
//! daemon ensure either); the next `laboratories spawn` converges
//! from config.

use objectiveai_sdk::cli::command::laboratories::config::local::set::{Request, Response};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub async fn execute(global: &GlobalContext, scoped: &ScopedContext, request: Request) -> Result<Response, Error> {
    let mut config = scoped.filesystem.read_config().await?;
    config.laboratories().set_local(request.value);
    scoped.filesystem.write_config(&config).await?;
    crate::command::laboratories::spawn::converge(global, scoped).await?;
    Ok(Response::Ok)
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::laboratories::config::local::set as sdk;
    use objectiveai_sdk::cli::command::laboratories::config::local::set::request_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::laboratories::config::local::set as sdk;
    use objectiveai_sdk::cli::command::laboratories::config::local::set::response_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
