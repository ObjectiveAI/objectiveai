//! `agents mcp resources list` — connect to the agent's per-`response_id`
//! MCP listener socket, run `resources/list`, and return the MCP
//! `ListResourcesResult`. A socket-level `err` reply or a connect/IO
//! failure surfaces as an `Error`.

use objectiveai_sdk::cli::command::agents::mcp::resources::list::{Request, Response};

use crate::context::Context;
use crate::error::Error;
use crate::websockets::mcp_listener::{SocketRequest, SocketResponse, call_socket};

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    let state_dir = ctx.filesystem.state_dir();
    let socket_request = SocketRequest::ListResources {
        params: request.params,
        name: request.name,
    };
    let response: SocketResponse<Response> =
        call_socket(&state_dir, &request.response_id, &socket_request)
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
    use objectiveai_sdk::cli::command::agents::mcp::resources::list as sdk;
    use objectiveai_sdk::cli::command::agents::mcp::resources::list::request_schema::{
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
    use objectiveai_sdk::cli::command::agents::mcp::resources::list as sdk;
    use objectiveai_sdk::cli::command::agents::mcp::resources::list::response_schema::{
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
