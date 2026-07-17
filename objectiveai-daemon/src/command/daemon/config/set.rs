//! `daemon config set` — FULL-REPLACE the whole `daemon` config
//! section from one wire object. The section's values are LINKED (a
//! secret and its derived signature, the address they authorize), so
//! per-field wire setters were retired: every mutation states one
//! complete consistent object, and omitted fields are cleared. The
//! stored section has no consumer yet (the daemon still binds from
//! bare env) — nothing to kill or respawn here until that wiring
//! lands.

use objectiveai_sdk::cli::command::daemon::config::set::{Request, Response};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub async fn execute(_global: &GlobalContext, scoped: &ScopedContext, request: Request) -> Result<Response, Error> {
    let mut config = scoped.filesystem.read_config().await?;
    config.daemon = Some(crate::filesystem::config::DaemonConfig {
        address: request.value.address,
        secret: request.value.secret,
        signature: request.value.signature,
    });
    scoped.filesystem.write_config(&config).await?;
    Ok(Response::Ok)
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::daemon::config::set as sdk;
    use objectiveai_sdk::cli::command::daemon::config::set::request_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::daemon::config::set as sdk;
    use objectiveai_sdk::cli::command::daemon::config::set::response_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
