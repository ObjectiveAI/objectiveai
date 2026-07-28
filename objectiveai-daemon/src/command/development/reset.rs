//! `development plugins mcp reset` — tell the LOCAL laboratory host to
//! drop a plugin's image so the next run rebuilds it.

use objectiveai_sdk::cli::command::development::plugins::mcp::reset::{Request, Response};
use objectiveai_sdk::laboratories::daemon::{
    JsonRpcResult, PluginImageResetRequest, RequestPayload, ResponsePayload,
};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub async fn execute(
    global: &GlobalContext,
    _scoped: &ScopedContext,
    request: Request,
) -> Result<Response, Error> {
    let hubs = global.resident_hubs().ok_or_else(|| {
        Error::Development(
            "development plugins mcp reset requires the resident daemon".to_string(),
        )
    })?;

    // The LOCAL host, never a random one. The image was built here, out
    // of a directory only this machine can see, so no other host has
    // anything to drop.
    let (machine, machine_state) = global.local_host();
    if !hubs.laboratories.has_host(machine, machine_state) {
        return Err(Error::Development(
            "no laboratory host is running for this machine/state — run \
             `laboratories spawn` first"
                .to_string(),
        ));
    }

    let key = super::registry::key(&request.owner, &request.name, &request.version);
    let payload = RequestPayload::PluginImageReset(PluginImageResetRequest {
        owner: key.0.clone(),
        name: key.1.clone(),
        version: key.2.clone(),
        caches: request.caches,
    });
    let response = hubs
        .laboratories
        .forward_to_host(machine, machine_state, indexmap::IndexMap::new(), payload)
        .await
        .map_err(Error::Development)?;

    match response {
        ResponsePayload::PluginImageReset(JsonRpcResult::Ok { result }) => Ok(Response {
            owner: key.0,
            name: key.1,
            version: key.2,
            removed: result.removed,
            caches_removed: u64::from(result.caches_removed),
        }),
        ResponsePayload::PluginImageReset(JsonRpcResult::Err { message, .. }) => {
            Err(Error::Development(message))
        }
        _ => Err(Error::Development(
            "laboratory host answered plugin image reset with an unexpected payload"
                .to_string(),
        )),
    }
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::development::plugins::mcp::reset as sdk;
    use objectiveai_sdk::cli::command::development::plugins::mcp::reset::request_schema::{
        Request, Response,
    };

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(
        _global: &GlobalContext,
        _scoped: &ScopedContext,
        _request: Request,
    ) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::Request),
        ))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::development::plugins::mcp::reset as sdk;
    use objectiveai_sdk::cli::command::development::plugins::mcp::reset::response_schema::{
        Request, Response,
    };

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(
        _global: &GlobalContext,
        _scoped: &ScopedContext,
        _request: Request,
    ) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::Response),
        ))
    }
}
