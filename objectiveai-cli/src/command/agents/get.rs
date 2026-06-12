//! `agents get` — read an agent definition by remote path.

use objectiveai_sdk::cli::command::agents::get::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    let path = request.path;
    Ok(objectiveai_sdk::agent::get_agent(ctx.api_client().await?, path).await?)
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::agents::get as sdk;
    use objectiveai_sdk::cli::command::agents::get::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::get as sdk;
    use objectiveai_sdk::cli::command::agents::get::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
