//! `daemon config signature get` — read `daemon.signature` from on-disk config.

use objectiveai_sdk::cli::command::daemon::config::signature::get::{Request, Response};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub async fn execute(_global: &GlobalContext, scoped: &ScopedContext, request: Request) -> Result<Response, Error> {
    let mut config = scoped.filesystem.read_config_view(request.scope).await?;
    Ok(Response {
        signature: config.daemon().get_signature().map(String::from),
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::daemon::config::signature::get as sdk;
    use objectiveai_sdk::cli::command::daemon::config::signature::get::request_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::daemon::config::signature::get as sdk;
    use objectiveai_sdk::cli::command::daemon::config::signature::get::response_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
