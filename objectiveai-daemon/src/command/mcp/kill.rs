//! `mcp kill --global|--state` — terminate mcp server(s) by
//! killing the owner(s) of their per-state lock at
//! `<dir>/state/<state>/locks` key `mcp`. `--state` hits the
//! current state; `--global` fans out across every state
//! concurrently. Idempotent: a count of zero is not an error.

use objectiveai_sdk::cli::command::mcp::kill::{Request, Response};

use crate::command::kill_helpers::kill_per_state;
use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    let killed = kill_per_state(ctx, request.scope, "mcp").await?;
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
