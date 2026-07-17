//! `daemon config get` — read the daemon section of on-disk config
//! (address + secret + signature). Missing fields stay `None`.

use objectiveai_sdk::cli::command::daemon::config::get::{Request, Response};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub async fn execute(_global: &GlobalContext, scoped: &ScopedContext, request: Request) -> Result<Response, Error> {
    let mut config = scoped.filesystem.read_config_view(request.scope).await?;
    let daemon = config.daemon();
    Ok(Response {
        address: daemon.get_address().map(String::from),
        secret: daemon.get_secret().map(String::from),
        signature: daemon.get_signature().map(String::from),
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::daemon::config::get as sdk;
    use objectiveai_sdk::cli::command::daemon::config::get::request_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::daemon::config::get as sdk;
    use objectiveai_sdk::cli::command::daemon::config::get::response_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
