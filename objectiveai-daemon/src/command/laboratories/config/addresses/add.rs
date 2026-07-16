//! `laboratories config addresses add` — add/replace one
//! `laboratories.addresses` entry in on-disk config. Both scopes are
//! allowed: the host is per-state, so its dial topology legitimately
//! layers (a global base of shared daemons, per-state extras).

use objectiveai_sdk::cli::command::laboratories::config::addresses::add::{Request, Response};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub async fn execute(_global: &GlobalContext, scoped: &ScopedContext, request: Request) -> Result<Response, Error> {
    let mut config = scoped.filesystem.read_config_at(request.scope).await?;
    config.laboratories().add_address(request.key, request.value);
    scoped.filesystem.write_config_at(request.scope, &config).await?;
    Ok(Response::Ok)
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::laboratories::config::addresses::add as sdk;
    use objectiveai_sdk::cli::command::laboratories::config::addresses::add::request_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::laboratories::config::addresses::add as sdk;
    use objectiveai_sdk::cli::command::laboratories::config::addresses::add::response_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
