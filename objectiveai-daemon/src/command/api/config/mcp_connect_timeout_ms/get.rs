//! `config api mcp-connect-timeout-ms get` — read `api.mcp_connect_timeout_ms` from on-disk config.

use objectiveai_sdk::cli::command::api::config::mcp_connect_timeout_ms::get::{Request, Response};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub async fn execute(_global: &GlobalContext, scoped: &ScopedContext, request: Request) -> Result<Response, Error> {
    let mut config = scoped.filesystem.read_config_view(request.scope).await?;
    Ok(Response {
        mcp_connect_timeout_ms: config.api().get_mcp_connect_timeout_ms(),
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::api::config::mcp_connect_timeout_ms::get as sdk;
    use objectiveai_sdk::cli::command::api::config::mcp_connect_timeout_ms::get::request_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::api::config::mcp_connect_timeout_ms::get as sdk;
    use objectiveai_sdk::cli::command::api::config::mcp_connect_timeout_ms::get::response_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
