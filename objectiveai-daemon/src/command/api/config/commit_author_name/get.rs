//! `config api commit-author-name get` — read `api.commit_author_name` from on-disk config.

use objectiveai_sdk::cli::command::api::config::commit_author_name::get::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    let mut config = ctx.filesystem.read_config_view(request.scope).await?;
    Ok(Response {
        commit_author_name: config.api().get_commit_author_name().map(String::from),
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::api::config::commit_author_name::get as sdk;
    use objectiveai_sdk::cli::command::api::config::commit_author_name::get::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::api::config::commit_author_name::get as sdk;
    use objectiveai_sdk::cli::command::api::config::commit_author_name::get::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
