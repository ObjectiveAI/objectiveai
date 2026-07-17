//! `laboratories config local set` — write `laboratories.local`,
//! reaching the RUNNING host live: resolve the local daemon's
//! coordinates exactly as the host spawn does (ensure the daemon,
//! signature from the daemon's OWN config), tell the host to add or
//! remove that address over its stdio dial-list channel (ack-gated;
//! both directions are idempotent), then write the config, and on a
//! write failure send the inverse command. No live host ⇒ write-only;
//! the next spawn seeds from config.

use objectiveai_sdk::cli::command::laboratories::config::local::set::{Request, Response};
use objectiveai_sdk::laboratories::daemon::HostStdioCommand;

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub async fn execute(global: &GlobalContext, scoped: &ScopedContext, request: Request) -> Result<Response, Error> {
    let mut config = scoped.filesystem.read_config().await?;
    let stdio = global.lab_host_stdio();
    // (command sent now, inverse for the revert path) — resolved only
    // when a live host needs telling.
    let commands = if let Some(stdio) = &stdio {
        let daemon_address = crate::command::daemon::spawn::spawn(global, scoped).await?;
        let add = HostStdioCommand::AddAddress {
            address: daemon_address.clone(),
            signature: global.client_signature(),
        };
        let remove = HostStdioCommand::RemoveAddress {
            address: daemon_address,
        };
        let (tell, revert) = if request.value {
            (add, remove)
        } else {
            (remove, add)
        };
        stdio.send_host_stdio(&tell).await?;
        Some(revert)
    } else {
        None
    };
    config.laboratories().set_local(request.value);
    if let Err(e) = scoped.filesystem.write_config(&config).await {
        if let (Some(stdio), Some(revert)) = (&stdio, commands) {
            // Best-effort: the write failure is the reportable error.
            let _ = stdio.send_host_stdio(&revert).await;
        }
        return Err(e.into());
    }
    Ok(Response::Ok)
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::laboratories::config::local::set as sdk;
    use objectiveai_sdk::cli::command::laboratories::config::local::set::request_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::laboratories::config::local::set as sdk;
    use objectiveai_sdk::cli::command::laboratories::config::local::set::response_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
