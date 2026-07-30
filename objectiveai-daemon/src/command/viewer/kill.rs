//! `viewer kill` — GRACEFULLY terminate this daemon's resident viewer
//! child. [`kill_resident_child`] recognizes the viewer by its stdio
//! channel and sends the acked
//! [`objectiveai_sdk::child_stdio::ChildStdioCommand::Shutdown`] over
//! its stdin, then waits (unbounded — no force path) for true exit:
//! the viewer closes every browser tab (persisting its profile to
//! disk) before exiting — and in development mode the graceful exit
//! of the innermost binary unwinds the whole `pnpm exec tauri dev`
//! tree. Idempotent: a count of zero is not an error.

use objectiveai_sdk::cli::command::viewer::kill::{Request, Response};

use crate::command::kill_helpers::kill_resident_child;
use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub async fn execute(global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
    let killed = kill_resident_child(global, "viewer").await;
    Ok(Response { killed })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::viewer::kill as sdk;
    use objectiveai_sdk::cli::command::viewer::kill::request_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::viewer::kill as sdk;
    use objectiveai_sdk::cli::command::viewer::kill::response_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
