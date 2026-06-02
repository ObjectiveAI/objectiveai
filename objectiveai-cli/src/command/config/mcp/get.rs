//! `config mcp get` — bare-naked handler stub.

use objectiveai_sdk::cli::command::config::mcp::get::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
    todo!("config mcp get execute")
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::config::mcp::get as sdk;
    use objectiveai_sdk::cli::command::config::mcp::get::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::config::mcp::get as sdk;
    use objectiveai_sdk::cli::command::config::mcp::get::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
