//! `laboratories config addresses del` — remove one
//! `laboratories.addresses` entry, reaching the RUNNING host live:
//! tell the host over its stdio dial-list channel first (ack-gated;
//! removing an absent address still acks), then write the config, and
//! on a write failure re-add the prior entry so the live dial list
//! matches disk. No live host ⇒ write-only.

use objectiveai_sdk::cli::command::laboratories::config::addresses::del::{Request, Response};
use objectiveai_sdk::laboratories::daemon::HostStdioCommand;

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub async fn execute(global: &GlobalContext, scoped: &ScopedContext, request: Request) -> Result<Response, Error> {
    let mut config = scoped.filesystem.read_config().await?;
    // The prior signature (empty = unauthenticated), captured for the
    // revert path. `None` means the key wasn't configured — the
    // remove is then a pure no-op on disk, but is still forwarded
    // (idempotent) so intent and dial list can't drift.
    let prior = config
        .laboratories
        .as_ref()
        .and_then(|labs| labs.get_addresses())
        .and_then(|addresses| addresses.get(&request.key))
        .cloned();
    let stdio = global.lab_host_stdio();
    if let Some(stdio) = &stdio {
        stdio
            .send_host_stdio(&HostStdioCommand::RemoveAddress {
                address: request.key.clone(),
            })
            .await?;
    }
    config.laboratories().del_address(&request.key);
    if let Err(e) = scoped.filesystem.write_config(&config).await {
        if let (Some(stdio), Some(signature)) = (&stdio, prior) {
            // Best-effort revert: restore the connection the config
            // still records.
            let _ = stdio
                .send_host_stdio(&HostStdioCommand::AddAddress {
                    address: request.key,
                    signature: (!signature.is_empty()).then_some(signature),
                })
                .await;
        }
        return Err(e.into());
    }
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
