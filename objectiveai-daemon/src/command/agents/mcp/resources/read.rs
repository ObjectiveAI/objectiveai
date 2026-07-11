//! `agents mcp resources read` — connect to the agent's per-`response_id`
//! MCP listener socket, run `resources/read`, and return the MCP
//! `ReadResourceResult`. A socket-level `err` reply or a connect/IO
//! failure surfaces as an `Error`.

use objectiveai_sdk::cli::command::agents::mcp::resources::read::{Request, Response};

use crate::context::Context;
use crate::error::Error;
use crate::websockets::mcp_listener::{SocketRequest, SocketResponse, call_notifier};

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    let socket_request = SocketRequest::ReadResource(request.params);
    let response: SocketResponse<Response> =
        call_notifier(ctx, &request.response_id, &socket_request)
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
    use objectiveai_sdk::cli::command::agents::mcp::resources::read as sdk;
    use objectiveai_sdk::cli::command::agents::mcp::resources::read::request_schema::{
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
    use objectiveai_sdk::cli::command::agents::mcp::resources::read as sdk;
    use objectiveai_sdk::cli::command::agents::mcp::resources::read::response_schema::{
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
