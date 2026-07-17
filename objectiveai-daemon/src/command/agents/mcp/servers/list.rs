//! `agents mcp servers list` — connect to the agent's per-`response_id` MCP
//! listener socket, run the proxy-local `servers/list` aggregate, and return
//! the connected upstream MCP servers + metadata. A socket-level `err` reply
//! or a connect/IO failure surfaces as an `Error`.

use objectiveai_sdk::cli::command::agents::mcp::servers::list::{Request, Response};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;
use crate::http::mcp_listener::{SocketRequest, SocketResponse, call_notifier};

pub async fn execute(global: &GlobalContext, scoped: &ScopedContext, request: Request) -> Result<Response, Error> {
    // `response_id` may be omitted: fall back to the invoking agent's
    // own response id from the contextual agent arguments
    // (`OBJECTIVEAI_RESPONSE_ID`, applied to the request scope —
    // unary and WebSocket alike).
    let Some(response_id) = request
        .response_id
        .clone()
        .or_else(|| scoped.response_id().map(String::from))
    else {
        return Err(Error::Instance(
            "no response_id given and none in the contextual agent arguments"
                .to_string(),
        ));
    };
    let socket_request = SocketRequest::ListServers;
    let response: SocketResponse<Response> =
        call_notifier(global, scoped, &response_id, &socket_request)
            .await
            .map_err(|e| Error::Instance(format!("mcp socket: {e}")))?;
    match response {
        SocketResponse::Ok(result) => Ok(result),
        SocketResponse::Err(e) => {
            Err(Error::Instance(format!("mcp error {}: {}", e.code, e.message)))
        }
    }
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::agents::mcp::servers::list as sdk;
    use objectiveai_sdk::cli::command::agents::mcp::servers::list::request_schema::{
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
    use objectiveai_sdk::cli::command::agents::mcp::servers::list as sdk;
    use objectiveai_sdk::cli::command::agents::mcp::servers::list::response_schema::{
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
