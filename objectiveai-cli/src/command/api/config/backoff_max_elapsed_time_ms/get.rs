//! `config api backoff-max-elapsed-time-ms get` — read `api.backoff_max_elapsed_time_ms` from on-disk config.

use objectiveai_sdk::cli::command::api::config::backoff_max_elapsed_time_ms::get::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    let mut config = ctx.filesystem.read_config_view(request.scope).await?;
    Ok(Response {
        backoff_max_elapsed_time_ms: config.api().get_backoff_max_elapsed_time_ms(),
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::api::config::backoff_max_elapsed_time_ms::get as sdk;
    use objectiveai_sdk::cli::command::api::config::backoff_max_elapsed_time_ms::get::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::api::config::backoff_max_elapsed_time_ms::get as sdk;
    use objectiveai_sdk::cli::command::api::config::backoff_max_elapsed_time_ms::get::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
