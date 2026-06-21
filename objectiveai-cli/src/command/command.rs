//! Root-level dispatch. Mirrors
//! `objectiveai-sdk-rs/src/cli/command/command.rs` — same `match` shape,
//! same fan-out, but each arm calls the *local* tier `execute` (which
//! actually does the work) instead of routing through `CommandExecutor`.
//!
//! `run.rs` calls into [`execute`] with the SDK's root `Request` after
//! parsing argv and running the SDK's `TryFrom<Command> for Request`.
//! `jq` is *not* applied here — the bare-naked command tree ignores the
//! `jq` field on incoming requests; `run.rs` is responsible for reading
//! it off the request and applying it to the stream we return.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

type ItemStream =
    Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

/// Shapes a unary `Result<T, E>` into a single-element stream so all
/// root arms can share one return type.
fn once<T: Send + 'static>(
    item: Result<T, Error>,
) -> Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>> {
    Box::pin(futures::stream::once(async move { item }))
}

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::Agents(req) => {
            let inner = super::agents::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Agents)))
        }
        Request::Api(req) => {
            let inner = super::api::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Api)))
        }
        Request::Daemon(req) => {
            let inner = super::daemon::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Daemon)))
        }
        Request::Db(req) => {
            let inner = super::db::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Db)))
        }
        Request::Functions(req) => {
            let inner = super::functions::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Functions)))
        }
        Request::KillAll(req) => {
            let value = super::kill_all::execute(ctx, req).await?;
            once(Ok(ResponseItem::KillAll(value)))
        }
        Request::KillAllRequestSchema(req) => {
            let value = super::kill_all::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::KillAllRequestSchema(value)))
        }
        Request::KillAllResponseSchema(req) => {
            let value = super::kill_all::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::KillAllResponseSchema(value)))
        }
        Request::Mcp(req) => {
            let inner = super::mcp::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Mcp)))
        }
        Request::Plugins(req) => {
            let inner = super::plugins::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Plugins)))
        }
        Request::Python(req) => {
            let value = super::python::execute(ctx, req).await?;
            once(Ok(ResponseItem::Python(value)))
        }
        Request::PythonRequestSchema(req) => {
            let value = super::python::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::PythonRequestSchema(value)))
        }
        Request::PythonResponseSchema(req) => {
            let value = super::python::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::PythonResponseSchema(value)))
        }
        Request::Swarms(req) => {
            let inner = super::swarms::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Swarms)))
        }
        Request::Tools(req) => {
            let inner = super::tools::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Tools)))
        }
        Request::Update(req) => {
            let inner = super::update::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Update)))
        }
        Request::UpdateRequestSchema(req) => {
            let value = super::update::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::UpdateRequestSchema(value)))
        }
        Request::UpdateResponseSchema(req) => {
            let value = super::update::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::UpdateResponseSchema(value)))
        }
        Request::Viewer(req) => {
            let inner = super::viewer::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Viewer)))
        }
    };
    Ok(stream)
}
