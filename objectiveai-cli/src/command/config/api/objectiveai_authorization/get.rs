//! `config api objectiveai-authorization get` — read `api.objectiveai_authorization` from on-disk config.

use objectiveai_sdk::cli::command::config::api::objectiveai_authorization::get::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, _request: Request) -> Result<Response, Error> {
    let mut config = ctx.filesystem.read_config().await?;
    Ok(Response {
        objectiveai_authorization: config.api().get_objectiveai_authorization().map(String::from),
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::config::api::objectiveai_authorization::get as sdk;
    use objectiveai_sdk::cli::command::config::api::objectiveai_authorization::get::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::config::api::objectiveai_authorization::get as sdk;
    use objectiveai_sdk::cli::command::config::api::objectiveai_authorization::get::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
