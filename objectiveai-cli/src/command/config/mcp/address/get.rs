//! `config mcp address get` — read `mcp.address` from on-disk config.

use objectiveai_sdk::cli::command::config::mcp::address::get::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, _request: Request) -> Result<Response, Error> {
    let mut config = ctx.filesystem.read_config().await?;
    Ok(Response {
        address: config.mcp().get_address().map(String::from),
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::config::mcp::address::get as sdk;
    use objectiveai_sdk::cli::command::config::mcp::address::get::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::config::mcp::address::get as sdk;
    use objectiveai_sdk::cli::command::config::mcp::address::get::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
