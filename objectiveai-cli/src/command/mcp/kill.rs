//! `mcp kill` — terminate every running `objectiveai-mcp` process.
//! Idempotent: a count of zero is not an error.

use objectiveai_sdk::cli::command::mcp::kill::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
    let killed = crate::spawn::kill_by_name("objectiveai-mcp");
    Ok(Response { killed })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::mcp::kill as sdk;
    use objectiveai_sdk::cli::command::mcp::kill::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::mcp::kill as sdk;
    use objectiveai_sdk::cli::command::mcp::kill::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
