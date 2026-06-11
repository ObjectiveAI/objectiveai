//! `config api http-referer get` — read `api.http_referer` from on-disk config.

use objectiveai_sdk::cli::command::config::api::http_referer::get::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, _request: Request) -> Result<Response, Error> {
    let mut config = ctx.filesystem.read_config().await?;
    Ok(Response {
        http_referer: config.api().get_http_referer().map(String::from),
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::config::api::http_referer::get as sdk;
    use objectiveai_sdk::cli::command::config::api::http_referer::get::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::config::api::http_referer::get as sdk;
    use objectiveai_sdk::cli::command::config::api::http_referer::get::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
