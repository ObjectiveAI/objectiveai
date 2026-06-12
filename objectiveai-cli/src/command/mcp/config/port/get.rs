//! `config mcp port get` — read `mcp.port` from on-disk config. The
//! SDK `Response.port` is non-optional, so an unset port becomes
//! `Error::MissingArgs("mcp.port unset")`.

use objectiveai_sdk::cli::command::mcp::config::port::get::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, _request: Request) -> Result<Response, Error> {
    let mut config = ctx.filesystem.read_config().await?;
    let port = config
        .mcp()
        .get_port()
        .ok_or(Error::MissingArgs("mcp.port unset"))?;
    Ok(Response { port })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::mcp::config::port::get as sdk;
    use objectiveai_sdk::cli::command::mcp::config::port::get::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::mcp::config::port::get as sdk;
    use objectiveai_sdk::cli::command::mcp::config::port::get::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
