//! `config api x-title get` — read `api.x_title` from on-disk config.

use objectiveai_sdk::cli::command::api::config::x_title::get::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, _request: Request) -> Result<Response, Error> {
    let mut config = ctx.filesystem.read_config().await?;
    Ok(Response {
        x_title: config.api().get_x_title().map(String::from),
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::api::config::x_title::get as sdk;
    use objectiveai_sdk::cli::command::api::config::x_title::get::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::api::config::x_title::get as sdk;
    use objectiveai_sdk::cli::command::api::config::x_title::get::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
