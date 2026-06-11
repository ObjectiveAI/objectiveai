//! `config api codex-sdk get` — read `api.codex_sdk` from on-disk config.

use objectiveai_sdk::cli::command::config::api::codex_sdk::get::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, _request: Request) -> Result<Response, Error> {
    let mut config = ctx.filesystem.read_config().await?;
    Ok(Response {
        codex_sdk: config.api().get_codex_sdk(),
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::config::api::codex_sdk::get as sdk;
    use objectiveai_sdk::cli::command::config::api::codex_sdk::get::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::config::api::codex_sdk::get as sdk;
    use objectiveai_sdk::cli::command::config::api::codex_sdk::get::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
