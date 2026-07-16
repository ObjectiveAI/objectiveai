//! `config mcp get` — read the mcp section of on-disk config (address
//! + port). Missing fields stay `None`.

use objectiveai_sdk::cli::command::mcp::config::get::{Request, Response};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub async fn execute(_global: &GlobalContext, scoped: &ScopedContext, request: Request) -> Result<Response, Error> {
    let mut config = scoped.filesystem.read_config_view(request.scope).await?;
    let mcp = config.mcp();
    Ok(Response {
        address: mcp.get_address().map(String::from),
        port: mcp.get_port(),
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::mcp::config::get as sdk;
    use objectiveai_sdk::cli::command::mcp::config::get::request_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::mcp::config::get as sdk;
    use objectiveai_sdk::cli::command::mcp::config::get::response_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
