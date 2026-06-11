//! `config api mcp-authorization add` — add/replace one `api.mcp_authorization` entry in on-disk config.

use objectiveai_sdk::cli::command::config::api::mcp_authorization::add::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    let mut config = ctx.filesystem.read_config().await?;
    config.api().add_mcp_authorization(request.key, request.value);
    ctx.filesystem.write_config(&config).await?;
    Ok(Response::Ok)
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::config::api::mcp_authorization::add as sdk;
    use objectiveai_sdk::cli::command::config::api::mcp_authorization::add::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::config::api::mcp_authorization::add as sdk;
    use objectiveai_sdk::cli::command::config::api::mcp_authorization::add::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
