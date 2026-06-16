//! `config api mcp-backoff get` — read `api.mcp_backoff` from on-disk config.

use objectiveai_sdk::cli::command::api::config::mcp_backoff::get::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    let mut config = ctx.filesystem.read_config_view(request.scope).await?;
    Ok(Response {
        mcp_backoff: config.api().get_mcp_backoff(),
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::api::config::mcp_backoff::get as sdk;
    use objectiveai_sdk::cli::command::api::config::mcp_backoff::get::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::api::config::mcp_backoff::get as sdk;
    use objectiveai_sdk::cli::command::api::config::mcp_backoff::get::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
