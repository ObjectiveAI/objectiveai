//! `laboratories list` — stream every laboratory served by a
//! connected laboratory HOST, straight from the daemon's `/laboratory`
//! registry. There is no local-vs-remote split and no podman scan —
//! hosts announce their full set on connect and notify on every
//! create/delete, so the registry IS the list; machine identity is the
//! only provenance, the same logic regardless of where a host runs.
//! The LOCAL host is auto-spawned first (best-effort, honoring
//! `laboratories config local: false` by skipping) so this machine's
//! laboratories appear without any prior command.
//! Read-only. Only client-side laboratories are supported today.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::laboratories::create::{EnvVar, Kind, Mount};
use objectiveai_sdk::cli::command::laboratories::list::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    // Only `Client` exists today; the match stays exhaustive so adding
    // `Server` later forces a decision here.
    match request.kind {
        Kind::Client => {}
    }

    // Best-effort local host: a fresh machine's laboratories should
    // list without a prior explicit spawn. `local: false` (or any
    // spawn failure — no binary, cold podman error) is NOT a list
    // error: the registry may still hold remote hosts.
    let _ = super::ensure_local_host(ctx).await;

    let labs = match ctx.resident_hubs() {
        Some(hubs) => hubs.laboratories.list().await,
        None => {
            return Err(Error::Laboratory(
                "laboratories list requires the resident daemon".to_string(),
            ));
        }
    };
    let stream = async_stream::stream! {
        for (machine, machine_state, lab) in labs {
            yield Ok(ResponseItem {
                id: lab.id,
                image: lab.image,
                mounts: lab
                    .mounts
                    .into_iter()
                    .map(|m| Mount {
                        host: m.host,
                        container: m.container,
                    })
                    .collect(),
                env: lab
                    .env
                    .into_iter()
                    .map(|[key, value]| EnvVar { key, value })
                    .collect(),
                cwd: lab.cwd,
                created_at: lab.created_at,
                agent_full_id: lab.agent_full_id,
                machine: Some(machine),
                machine_state: Some(machine_state),
                running: lab.running,
            });
        }
    };
    Ok(Box::pin(stream))
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::laboratories::list as sdk;
    use objectiveai_sdk::cli::command::laboratories::list::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::Request),
        ))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::laboratories::list as sdk;
    use objectiveai_sdk::cli::command::laboratories::list::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::ResponseItem),
        ))
    }
}
