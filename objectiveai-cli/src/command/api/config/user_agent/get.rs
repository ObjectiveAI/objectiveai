//! `config api user-agent get` — read `api.user_agent` from on-disk config.

use objectiveai_sdk::cli::command::api::config::user_agent::get::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, _request: Request) -> Result<Response, Error> {
    let mut config = ctx.filesystem.read_config().await?;
    Ok(Response {
        user_agent: config.api().get_user_agent().map(String::from),
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::api::config::user_agent::get as sdk;
    use objectiveai_sdk::cli::command::api::config::user_agent::get::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::api::config::user_agent::get as sdk;
    use objectiveai_sdk::cli::command::api::config::user_agent::get::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
