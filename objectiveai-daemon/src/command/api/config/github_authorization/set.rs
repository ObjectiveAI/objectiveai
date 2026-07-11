//! `config api github-authorization set` — write `api.github_authorization` to on-disk config.

use objectiveai_sdk::cli::command::api::config::github_authorization::set::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    if !matches!(
        request.scope,
        objectiveai_sdk::cli::command::SetScope::Global
    ) {
        return Err(Error::AuthorizationGlobalOnly);
    }
    let mut config = ctx.filesystem.read_config_at(request.scope).await?;
    config.api().set_github_authorization(request.value);
    ctx.filesystem.write_config_at(request.scope, &config).await?;
    Ok(Response::Ok)
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::api::config::github_authorization::set as sdk;
    use objectiveai_sdk::cli::command::api::config::github_authorization::set::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::api::config::github_authorization::set as sdk;
    use objectiveai_sdk::cli::command::api::config::github_authorization::set::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
