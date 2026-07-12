//! `laboratories create` — create the laboratory container on the
//! TARGET MACHINE's laboratory host, forwarded over its `/laboratory`
//! WS (`LaboratoryCreate`): podman runs host-side, wherever the host
//! is. `--machine` picks the exact machine id (from `laboratories
//! list`); unset targets the current machine, auto-spawning its host
//! when none is connected (unless `laboratories config local` is
//! false — then this errors, as the local host would never dial this
//! daemon). The container is NOT started — it starts lazily on its
//! first routed op. The host broadcasts `laboratory_created` to every
//! daemon it serves, so all registries update without scanning. The
//! echo is the host's own reply (podman's record, `created_at`
//! included). Errors if the id already exists on that host. Only
//! client-side laboratories are supported today.

use objectiveai_sdk::cli::command::laboratories::create::{Kind, Request, Response};
use objectiveai_sdk::client_objectiveai_mcp::server_response::JsonRpcResult;
use objectiveai_sdk::client_objectiveai_mcp::{server_request, server_response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    // Only `Client` exists today; the match stays exhaustive so adding
    // `Server` later forces a decision here.
    match request.kind {
        Kind::Client => {}
    }

    let target = super::resolve_machine(ctx, request.machine.clone()).await?;
    let hubs = ctx
        .resident_hubs()
        .ok_or_else(|| Error::Laboratory("laboratories create requires the resident daemon".to_string()))?;

    let payload = server_request::Payload::LaboratoryCreate(
        server_request::LaboratoryCreateRequest {
            id: request.id.clone(),
            image: request.image.clone(),
            mounts: request
                .mounts
                .iter()
                .map(|m| [m.host.clone(), m.container.clone()])
                .collect(),
            env: request
                .env
                .iter()
                .map(|e| [e.key.clone(), e.value.clone()])
                .collect(),
            cwd: request.cwd.clone(),
        },
    );
    let response = hubs
        .laboratories
        .forward_to_machine(&target, indexmap::IndexMap::new(), payload)
        .await
        .map_err(Error::Laboratory)?;
    let identify = match response {
        server_response::Payload::LaboratoryCreate(JsonRpcResult::Ok { result }) => result,
        server_response::Payload::LaboratoryCreate(JsonRpcResult::Err {
            message, ..
        }) => return Err(Error::Laboratory(message)),
        _ => {
            return Err(Error::Laboratory(
                "laboratory host answered create with an unexpected payload".to_string(),
            ));
        }
    };
    Ok(Response {
        id: identify.id,
        image: identify.image,
        mounts: identify
            .mounts
            .into_iter()
            .map(|m| objectiveai_sdk::cli::command::laboratories::create::Mount {
                host: m.host,
                container: m.container,
            })
            .collect(),
        env: identify
            .env
            .into_iter()
            .map(|[key, value]| {
                objectiveai_sdk::cli::command::laboratories::create::EnvVar { key, value }
            })
            .collect(),
        cwd: identify.cwd,
        created_at: identify.created_at,
        machine: hubs.laboratories.machine(&target),
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::laboratories::create as sdk;
    use objectiveai_sdk::cli::command::laboratories::create::request_schema::{
        Request, Response,
    };

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::Request),
        ))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::laboratories::create as sdk;
    use objectiveai_sdk::cli::command::laboratories::create::response_schema::{
        Request, Response,
    };

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::Response),
        ))
    }
}
