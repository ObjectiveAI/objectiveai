//! `config api openrouter-authorization get` — read `api.openrouter_authorization` from on-disk config.

use objectiveai_sdk::cli::command::api::config::openrouter_authorization::get::{Request, Response};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub async fn execute(_global: &GlobalContext, scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
    let mut config = scoped.filesystem.read_config().await?;
    Ok(Response {
        openrouter_authorization: config.api().get_openrouter_authorization().map(String::from),
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::api::config::openrouter_authorization::get as sdk;
    use objectiveai_sdk::cli::command::api::config::openrouter_authorization::get::request_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::api::config::openrouter_authorization::get as sdk;
    use objectiveai_sdk::cli::command::api::config::openrouter_authorization::get::response_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
