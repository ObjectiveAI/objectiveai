//! `config api http-referer get` — read `api.http_referer` from on-disk config.

use objectiveai_sdk::cli::command::api::config::http_referer::get::{Request, Response};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub async fn execute(_global: &GlobalContext, scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
    let mut config = scoped.filesystem.read_config().await?;
    Ok(Response {
        http_referer: config.api().get_http_referer().map(String::from),
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::api::config::http_referer::get as sdk;
    use objectiveai_sdk::cli::command::api::config::http_referer::get::request_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::api::config::http_referer::get as sdk;
    use objectiveai_sdk::cli::command::api::config::http_referer::get::response_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
