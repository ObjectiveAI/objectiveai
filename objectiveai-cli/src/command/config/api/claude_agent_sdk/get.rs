//! `config api claude-agent-sdk get` — read `api.claude_agent_sdk` from on-disk config.

use objectiveai_sdk::cli::command::config::api::claude_agent_sdk::get::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, _request: Request) -> Result<Response, Error> {
    let mut config = ctx.filesystem.read_config().await?;
    Ok(Response {
        claude_agent_sdk: config.api().get_claude_agent_sdk(),
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::config::api::claude_agent_sdk::get as sdk;
    use objectiveai_sdk::cli::command::config::api::claude_agent_sdk::get::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::config::api::claude_agent_sdk::get as sdk;
    use objectiveai_sdk::cli::command::config::api::claude_agent_sdk::get::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
