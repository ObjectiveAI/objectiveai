//! `config api mcp-authorization del` — remove one `api.mcp_authorization` entry from on-disk config.

use objectiveai_sdk::cli::command::api::config::mcp_authorization::del::{Request, Response};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub async fn execute(global: &GlobalContext, scoped: &ScopedContext, request: Request) -> Result<Response, Error> {
    // Retire the running api server BEFORE the write: it was spawned
    // with the config this set replaces, and a server we cannot kill
    // must not survive the change — so a kill failure aborts the set.
    crate::command::kill_helpers::kill_api_before_config_change(global).await?;
    let mut config = scoped.filesystem.read_config().await?;
    config.api().del_mcp_authorization(&request.key);
    scoped.filesystem.write_config(&config).await?;
    // Sweep again after the write (best-effort): a concurrent request
    // may have respawned the api against the OLD config mid-set.
    crate::command::kill_helpers::kill_api_after_config_change(global).await;
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
