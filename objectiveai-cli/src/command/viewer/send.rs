//! `viewer send <path> <body>` — POST a JSON body to the viewer's HTTP
//! server and return its `(status, body)`. Bypasses the SDK's
//! fire-and-forget viewer client because callers want to see the
//! response synchronously.
//!
//! Address/signature resolution lives in `Context::viewer_client()`
//! (config `viewer.address`, else the `viewer spawn` flow's published
//! lock URL; signature env `VIEWER_SIGNATURE`, else
//! `viewer.signature`). Non-2xx status codes are not an error; the
//! response is returned as-is with the status set on the `Response`.

use objectiveai_sdk::cli::command::viewer::send::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    let Request { path, body, .. } = request;
    let (status, body) = ctx.viewer_client().await?.send(&path, &body).await?;
    Ok(Response { status, body })
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
