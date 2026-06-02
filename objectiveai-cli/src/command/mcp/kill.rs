//! `mcp kill` — bare-naked handler stub.

use objectiveai_sdk::cli::command::mcp::kill::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
    todo!("mcp kill execute")
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
