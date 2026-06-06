//! `config mcp get` — read the mcp section of on-disk config (address
//! + port). Missing fields stay `None`.

use objectiveai_sdk::cli::command::config::mcp::get::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, _request: Request) -> Result<Response, Error> {
    let mut config = ctx.filesystem.read_config().await?;
    let mcp = config.mcp();
    Ok(Response {
        address: mcp.get_address().map(String::from),
        port: mcp.get_port(),
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::config::mcp::get as sdk;
    use objectiveai_sdk::cli::command::config::mcp::get::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::config::mcp::get as sdk;
    use objectiveai_sdk::cli::command::config::mcp::get::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
