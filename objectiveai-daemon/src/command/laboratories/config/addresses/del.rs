//! `laboratories config addresses del` — remove one
//! `laboratories.addresses` entry: write the config (the desired
//! state), then CONVERGE a running host to it
//! ([`crate::command::laboratories::spawn::converge`] — ack-gated;
//! deleting the LAST address legitimately converges the host to an
//! empty dial list and it idles). No live host ⇒ the converge no-ops
//! immediately; the next `laboratories spawn` converges from config.

use objectiveai_sdk::cli::command::laboratories::config::addresses::del::{Request, Response};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub async fn execute(global: &GlobalContext, scoped: &ScopedContext, request: Request) -> Result<Response, Error> {
    let mut config = scoped.filesystem.read_config().await?;
    config.laboratories().del_address(&request.key);
    scoped.filesystem.write_config(&config).await?;
    crate::command::laboratories::spawn::converge(global, scoped).await?;
    Ok(Response::Ok)
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::laboratories::config::addresses::del as sdk;
    use objectiveai_sdk::cli::command::laboratories::config::addresses::del::request_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::laboratories::config::addresses::del as sdk;
    use objectiveai_sdk::cli::command::laboratories::config::addresses::del::response_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
