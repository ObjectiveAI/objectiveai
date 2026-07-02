//! `viewer send <path> <body>` — acknowledge and return immediately.
//!
//! It no longer POSTs to the viewer's HTTP server: the request is
//! broadcast to the viewer over the daemon WebSocket, so there's nothing
//! to send synchronously here. The handler just returns the shared `Ok`
//! sentinel.

use objectiveai_sdk::cli::command::viewer::send::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
    Ok(Response::Ok)
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::viewer::send as sdk;
    use objectiveai_sdk::cli::command::viewer::send::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::viewer::send as sdk;
    use objectiveai_sdk::cli::command::viewer::send::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
