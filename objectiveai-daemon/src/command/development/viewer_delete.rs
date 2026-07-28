//! `development plugins viewer delete` — drop a viewer development
//! registration and tell the running viewer, which falls back to
//! the installed copy.

use objectiveai_sdk::cli::command::development::plugins::viewer::delete::{Request, Response};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub async fn execute(
    global: &GlobalContext,
    _scoped: &ScopedContext,
    request: Request,
) -> Result<Response, Error> {
    let hubs = global.resident_hubs().ok_or_else(|| {
        Error::Development(
            "development plugins viewer delete requires the resident daemon".to_string(),
        )
    })?;

    let key = super::registry::key(&request.owner, &request.name, &request.version);
    // Removing nothing is a success: the caller asked for this plugin
    // not to be in development mode, and it is not.
    let removed = hubs.development_plugins.viewer.remove(&key).is_some();

    // Soft, same as create: nobody listening is a valid state.
    super::viewer_converge::viewer_converge(global).await?;

    Ok(Response {
        owner: key.0,
        name: key.1,
        version: key.2,
        removed,
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::development::plugins::viewer::delete as sdk;
    use objectiveai_sdk::cli::command::development::plugins::viewer::delete::request_schema::{
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
    use objectiveai_sdk::cli::command::development::plugins::viewer::delete as sdk;
    use objectiveai_sdk::cli::command::development::plugins::viewer::delete::response_schema::{
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
