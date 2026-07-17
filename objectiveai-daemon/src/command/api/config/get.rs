//! `config api get` — read the api section of on-disk config (every
//! `ApiConfig` field). Missing fields stay `None`.

use objectiveai_sdk::cli::command::api::config::get::{Request, Response};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub async fn execute(_global: &GlobalContext, scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
    let mut config = scoped.filesystem.read_config().await?;
    let api = config.api();
    Ok(Response {
        address: api.get_address().map(String::from),
        objectiveai_authorization: api.get_objectiveai_authorization().map(String::from),
        openrouter_authorization: api.get_openrouter_authorization().map(String::from),
        github_authorization: api.get_github_authorization().map(String::from),
        mcp_authorization: api.get_mcp_authorization().cloned(),
        user_agent: api.get_user_agent().map(String::from),
        http_referer: api.get_http_referer().map(String::from),
        x_title: api.get_x_title().map(String::from),
        commit_author_name: api.get_commit_author_name().map(String::from),
        commit_author_email: api.get_commit_author_email().map(String::from),
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::api::config::get as sdk;
    use objectiveai_sdk::cli::command::api::config::get::request_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::api::config::get as sdk;
    use objectiveai_sdk::cli::command::api::config::get::response_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
