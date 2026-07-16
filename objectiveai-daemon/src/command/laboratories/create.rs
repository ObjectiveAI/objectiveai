//! `laboratories create` — create the laboratory container on the
//! TARGET host, forwarded over its `/laboratory` WS
//! (`LaboratoryCreate`): podman runs host-side, wherever the host is.
//! `--machine` + `--machine-state` pick the exact host (both or
//! neither); neither targets (the current machine, the daemon's own
//! state), auto-spawning its host when none is connected (unless
//! `laboratories config local` is false — then this errors, as the
//! local host would never dial this daemon). The container is NOT
//! started — it starts lazily on its first routed op. The host
//! broadcasts `laboratory_created` to every daemon it serves, so all
//! registries update without scanning. The echo is the host's own
//! reply (podman's record, `created_at` included). Errors ONLY if the
//! id already exists on THAT host — the same id on a different
//! laboratory daemon is fine (ids are only unique per (machine,
//! state); there is deliberately no cross-host duplicate check).
//! Only client-side laboratories are supported today.

use objectiveai_sdk::cli::command::laboratories::create::{Kind, Request, Response};
use objectiveai_sdk::laboratories::daemon::{JsonRpcResult, RequestPayload, ResponsePayload};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    // Only `Client` exists today; the match stays exhaustive so adding
    // `Server` later forces a decision here.
    match request.kind {
        Kind::Client => {}
    }

    // Early, friendly validation — the host re-checks authoritatively.
    request
        .image
        .validate()
        .map_err(|message| Error::Laboratory(format!("image: {message}")))?;

    // The `oai-agent-` namespace is reserved: agent laboratories are
    // derived from agent definitions and created by the conduit at
    // MCP-initialize, never by `laboratories create`. (The host
    // re-checks authoritatively.)
    if request
        .id
        .starts_with(objectiveai_sdk::agent::AGENT_LABORATORY_ID_PREFIX)
    {
        return Err(Error::Laboratory(format!(
            "laboratory id '{}' uses the reserved agent-laboratory prefix '{}'",
            request.id,
            objectiveai_sdk::agent::AGENT_LABORATORY_ID_PREFIX,
        )));
    }

    // Ids are one URL path segment (`/laboratories/{id}`,
    // `/laboratories/{id}/filetree`) — a `/` would break the routes.
    // (The host re-checks authoritatively.)
    if request.id.contains('/') {
        return Err(Error::Laboratory(format!(
            "laboratory id '{}' contains '/' — ids must be a single path segment",
            request.id,
        )));
    }

    let (target, host_state) =
        super::resolve_pair(ctx, request.machine.clone(), request.machine_state.clone())?;
    super::ensure_host(ctx, &target, &host_state).await?;
    let hubs = ctx
        .resident_hubs()
        .ok_or_else(|| Error::Laboratory("laboratories create requires the resident daemon".to_string()))?;

    let payload = RequestPayload::Create(
        objectiveai_sdk::laboratories::daemon::CreateRequest {
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
            agent_full_id: None,
        },
    );
    let response = hubs
        .laboratories
        .forward_to_host(&target, &host_state, indexmap::IndexMap::new(), payload)
        .await
        .map_err(Error::Laboratory)?;
    let identify = match response {
        ResponsePayload::Create(JsonRpcResult::Ok { result }) => result,
        ResponsePayload::Create(JsonRpcResult::Err {
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
        machine: hubs.laboratories.machine(&target, &host_state),
        machine_state: Some(host_state),
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
