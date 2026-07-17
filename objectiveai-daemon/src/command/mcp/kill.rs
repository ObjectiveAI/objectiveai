//! `mcp kill` — terminate this daemon's resident mcp child.
//! Idempotent: a count of zero is not an error.

use objectiveai_sdk::cli::command::mcp::kill::{Request, Response};

use crate::command::kill_helpers::kill_resident_child;
use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub async fn execute(global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
    let killed = kill_resident_child(global, "mcp").await;
    Ok(Response { killed })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::mcp::kill as sdk;
    use objectiveai_sdk::cli::command::mcp::kill::request_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::mcp::kill as sdk;
    use objectiveai_sdk::cli::command::mcp::kill::response_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
