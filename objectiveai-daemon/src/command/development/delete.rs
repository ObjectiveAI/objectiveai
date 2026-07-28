//! `development plugins mcp delete` — drop a development registration.

use objectiveai_sdk::cli::command::development::plugins::mcp::delete::{Request, Response};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub async fn execute(
    global: &GlobalContext,
    _scoped: &ScopedContext,
    request: Request,
) -> Result<Response, Error> {
    let hubs = global.resident_hubs().ok_or_else(|| {
        Error::Development(
            "development plugins mcp delete requires the resident daemon".to_string(),
        )
    })?;

    let key = super::registry::key(&request.owner, &request.name, &request.version);
    // Removing nothing is a success: the caller asked for this plugin
    // not to be in development mode, and it is not.
    let removed = hubs.development_plugins.remove(&key).is_some();

    Ok(Response {
        owner: key.0,
        name: key.1,
        version: key.2,
        removed,
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::development::plugins::mcp::delete as sdk;
    use objectiveai_sdk::cli::command::development::plugins::mcp::delete::request_schema::{
        Request, Response,
    };

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(
        _global: &GlobalContext,
        _scoped: &ScopedContext,
        _request: Request,
    ) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::Request),
        ))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::development::plugins::mcp::delete as sdk;
    use objectiveai_sdk::cli::command::development::plugins::mcp::delete::response_schema::{
        Request, Response,
    };

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(
        _global: &GlobalContext,
        _scoped: &ScopedContext,
        _request: Request,
    ) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::Response),
        ))
    }
}
