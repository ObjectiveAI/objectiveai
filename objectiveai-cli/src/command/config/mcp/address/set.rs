//! `config mcp address set` — bare-naked handler stub.

use objectiveai_sdk::cli::command::config::mcp::address::set::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
    todo!("config mcp address set execute")
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::config::mcp::address::set as sdk;
    use objectiveai_sdk::cli::command::config::mcp::address::set::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::config::mcp::address::set as sdk;
    use objectiveai_sdk::cli::command::config::mcp::address::set::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
