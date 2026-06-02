//! `config mcp port get` — bare-naked handler stub.

use objectiveai_sdk::cli::command::config::mcp::port::get::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
    todo!("config mcp port get execute")
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::config::mcp::port::get as sdk;
    use objectiveai_sdk::cli::command::config::mcp::port::get::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::config::mcp::port::get as sdk;
    use objectiveai_sdk::cli::command::config::mcp::port::get::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
