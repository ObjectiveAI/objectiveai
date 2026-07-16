//! `config api mcp-authorization del` — remove one `api.mcp_authorization` entry from on-disk config.

use objectiveai_sdk::cli::command::api::config::mcp_authorization::del::{Request, Response};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub async fn execute(_global: &GlobalContext, scoped: &ScopedContext, request: Request) -> Result<Response, Error> {
    if !matches!(
        request.scope,
        objectiveai_sdk::cli::command::SetScope::Global
    ) {
        return Err(Error::AuthorizationGlobalOnly);
    }
    let mut config = scoped.filesystem.read_config_at(request.scope).await?;
    config.api().del_mcp_authorization(&request.key);
    scoped.filesystem.write_config_at(request.scope, &config).await?;
    Ok(Response::Ok)
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::api::config::mcp_authorization::del as sdk;
    use objectiveai_sdk::cli::command::api::config::mcp_authorization::del::request_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::api::config::mcp_authorization::del as sdk;
    use objectiveai_sdk::cli::command::api::config::mcp_authorization::del::response_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
