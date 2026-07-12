//! `laboratories delete` — remove the laboratory container on
//! whichever machine's host serves it, forwarded over its
//! `/laboratory` WS (`LaboratoryDelete` → host-side podman `rm -f`,
//! force-removing even a running container, reclaiming disk; a missing
//! container is not an error). Routing is by laboratory id — no
//! machine argument, no local-vs-remote. When NO connected host serves
//! the id, the local host is auto-spawned once (unless `laboratories
//! config local` is false) in case the laboratory lives here
//! unannounced; still unserved after that is an error. The host
//! broadcasts `laboratory_deleted`, so every daemon's registry
//! updates without scanning. Only client-side laboratories are
//! supported today.

use objectiveai_sdk::cli::command::laboratories::delete::{Kind, Request, Response};
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

    let hubs = ctx
        .resident_hubs()
        .ok_or_else(|| Error::Laboratory("laboratories delete requires the resident daemon".to_string()))?;

    // No connected host serves the id → the laboratory may live on
    // THIS machine with its host not yet spawned. One auto-spawn
    // (honoring `local: false` by erroring), then re-check.
    if hubs
        .laboratories
        .machine_for_laboratory(&request.id)
        .await
        .is_none()
    {
        super::ensure_local_host(ctx).await?;
        if hubs
            .laboratories
            .machine_for_laboratory(&request.id)
            .await
            .is_none()
        {
            return Err(Error::Laboratory(format!(
                "laboratory '{}' is not served by any connected host",
                request.id
            )));
        }
    }

    let payload = server_request::Payload::LaboratoryDelete(
        server_request::LaboratoryDeleteRequest { id: request.id.clone() },
    );
    let response = hubs
        .laboratories
        .forward(&request.id, indexmap::IndexMap::new(), payload)
        .await
        .map_err(Error::Laboratory)?;
    match response {
        server_response::Payload::LaboratoryDelete(JsonRpcResult::Ok { .. }) => {
            Ok(Response { id: request.id })
        }
        server_response::Payload::LaboratoryDelete(JsonRpcResult::Err {
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

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
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

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::Response),
        ))
    }
}
