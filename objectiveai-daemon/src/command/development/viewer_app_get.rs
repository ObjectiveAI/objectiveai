//! `development viewer get` — the current viewer-source registration.

use objectiveai_sdk::cli::command::development::viewer::get::{Request, Response};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub async fn execute(
    global: &GlobalContext,
    _scoped: &ScopedContext,
    _request: Request,
) -> Result<Response, Error> {
    let hubs = global.resident_hubs().ok_or_else(|| {
        Error::Development(
            "development viewer get requires the resident daemon".to_string(),
        )
    })?;
    Ok(Response {
        path: hubs
            .development_plugins
            .viewer_app
            .get()
            .map(|path| path.to_string_lossy().into_owned()),
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::development::viewer::get as sdk;
    use objectiveai_sdk::cli::command::development::viewer::get::request_schema::{
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
    use objectiveai_sdk::cli::command::development::viewer::get as sdk;
    use objectiveai_sdk::cli::command::development::viewer::get::response_schema::{
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
