//! `laboratories delete` — remove the laboratory container on the
//! TARGET host, forwarded over its `/laboratory` WS
//! (`LaboratoryDelete` → host-side podman `rm -f`, force-removing
//! even a running container, reclaiming disk; a missing container is
//! not an error). `--machine` + `--machine-state` pick the exact host
//! (both or neither); neither targets (the current machine, the
//! daemon's own state), auto-spawning its host when none is connected
//! (unless `laboratories config local` is false). There is NO
//! route-by-bare-id scan — laboratory ids are only unique per
//! (machine, state), so the pair IS the address. The host broadcasts
//! `laboratory_deleted`, so every daemon's registry updates without
//! scanning. Only client-side laboratories are supported today.

use objectiveai_sdk::cli::command::laboratories::delete::{Kind, Request, Response};
use objectiveai_sdk::laboratories::daemon::{JsonRpcResult, RequestPayload, ResponsePayload};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub async fn execute(global: &GlobalContext, scoped: &ScopedContext, request: Request) -> Result<Response, Error> {
    // Only `Client` exists today; the match stays exhaustive so adding
    // `Server` later forces a decision here.
    match request.kind {
        Kind::Client => {}
    }

    let (machine, machine_state) =
        super::resolve_pair(global, scoped, request.machine.clone(), request.machine_state.clone())?;
    super::ensure_host(global, scoped, &machine, &machine_state).await?;
    let hubs = global
        .resident_hubs()
        .ok_or_else(|| Error::Laboratory("laboratories delete requires the resident daemon".to_string()))?;

    let payload = RequestPayload::Delete(
        objectiveai_sdk::laboratories::daemon::DeleteRequest { id: request.id.clone() },
    );
    let response = hubs
        .laboratories
        .forward_to_host(&machine, &machine_state, indexmap::IndexMap::new(), payload)
        .await
        .map_err(Error::Laboratory)?;
    match response {
        ResponsePayload::Delete(JsonRpcResult::Ok { .. }) => {
            Ok(Response { id: request.id })
        }
        ResponsePayload::Delete(JsonRpcResult::Err {
            message, ..
        }) => Err(Error::Laboratory(message)),
        _ => Err(Error::Laboratory(
            "laboratory host answered delete with an unexpected payload".to_string(),
        )),
    }
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::laboratories::delete as sdk;
    use objectiveai_sdk::cli::command::laboratories::delete::request_schema::{
        Request, Response,
    };

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::Request),
        ))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::laboratories::delete as sdk;
    use objectiveai_sdk::cli::command::laboratories::delete::response_schema::{
        Request, Response,
    };

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::Response),
        ))
    }
}
