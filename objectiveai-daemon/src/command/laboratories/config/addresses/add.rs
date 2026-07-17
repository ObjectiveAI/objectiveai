//! `laboratories config addresses add` — add/replace one
//! `laboratories.addresses` entry, reaching the RUNNING host live:
//! tell the host over its stdio dial-list channel first (ack-gated —
//! a host that can't apply the change leaves the config untouched),
//! then write the config, and on a write failure tell the host to
//! revert (restore the prior signature if the key existed, remove
//! otherwise) so the live dial list matches disk. No live host ⇒
//! write-only; the next spawn seeds from config.

use objectiveai_sdk::cli::command::laboratories::config::addresses::add::{Request, Response};
use objectiveai_sdk::laboratories::daemon::HostStdioCommand;

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub async fn execute(global: &GlobalContext, scoped: &ScopedContext, request: Request) -> Result<Response, Error> {
    let mut config = scoped.filesystem.read_config().await?;
    // The prior entry (an EMPTY value means "dial unauthenticated"),
    // captured before the overwrite for the revert path.
    let prior = config
        .laboratories
        .as_ref()
        .and_then(|labs| labs.get_addresses())
        .and_then(|addresses| addresses.get(&request.key))
        .cloned();
    let stdio = global.lab_host_stdio();
    if let Some(stdio) = &stdio {
        stdio
            .send_host_stdio(&HostStdioCommand::AddAddress {
                address: request.key.clone(),
                signature: (!request.value.is_empty()).then(|| request.value.clone()),
            })
            .await?;
    }
    config
        .laboratories()
        .add_address(request.key.clone(), request.value);
    if let Err(e) = scoped.filesystem.write_config(&config).await {
        if let Some(stdio) = &stdio {
            let revert = match prior {
                Some(signature) => HostStdioCommand::AddAddress {
                    address: request.key,
                    signature: (!signature.is_empty()).then_some(signature),
                },
                None => HostStdioCommand::RemoveAddress {
                    address: request.key,
                },
            };
            // Best-effort: the write already failed and that error is
            // the one worth reporting.
            let _ = stdio.send_host_stdio(&revert).await;
        }
        return Err(e.into());
    }
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
