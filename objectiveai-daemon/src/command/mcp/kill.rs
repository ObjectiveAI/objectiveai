//! `mcp kill` — terminate this daemon's resident mcp child.
//! Idempotent: a count of zero is not an error.

use objectiveai_sdk::cli::command::mcp::kill::{Request, Response};

use crate::command::kill_helpers::kill_resident_child;
use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    // `scope` is vestigial: other states' servers belong to other
    // daemons and die with them — both scopes mean this daemon's
    // resident child.
    let _ = request.scope;
    let killed = kill_resident_child(ctx, "mcp").await;
    Ok(Response { killed })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::mcp::kill as sdk;
    use objectiveai_sdk::cli::command::mcp::kill::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::mcp::kill as sdk;
    use objectiveai_sdk::cli::command::mcp::kill::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
