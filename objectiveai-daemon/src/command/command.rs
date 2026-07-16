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

use crate::context::{GlobalContext, ScopedContext};
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

pub async fn execute(global: &GlobalContext, scoped: &ScopedContext, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::Agents(req) => {
            let inner = super::agents::execute(global, scoped, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Agents)))
        }
        Request::Api(req) => {
            let inner = super::api::execute(global, scoped, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Api)))
        }
        Request::Daemon(req) => {
            let inner = super::daemon::execute(global, scoped, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Daemon)))
        }
        Request::Db(req) => {
            let inner = super::db::execute(global, scoped, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Db)))
        }
        Request::Functions(req) => {
            let inner = super::functions::execute(global, scoped, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Functions)))
        }
        Request::Laboratories(req) => {
            let inner = super::laboratories::execute(global, scoped, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Laboratories)))
        }
        Request::Mcp(req) => {
            let inner = super::mcp::execute(global, scoped, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Mcp)))
        }
        Request::Plugins(req) => {
            let inner = super::plugins::execute(global, scoped, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Plugins)))
        }
        Request::Python(req) => {
            let value = super::python::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::Python(value)))
        }
        Request::PythonRequestSchema(req) => {
            let value = super::python::request_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::PythonRequestSchema(value)))
        }
        Request::PythonResponseSchema(req) => {
            let value = super::python::response_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::PythonResponseSchema(value)))
        }
        Request::Swarms(req) => {
            let inner = super::swarms::execute(global, scoped, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Swarms)))
        }
        Request::Tools(req) => {
            let inner = super::tools::execute(global, scoped, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Tools)))
        }
        Request::Update(req) => {
            let inner = super::update::execute(global, scoped, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Update)))
        }
        Request::UpdateRequestSchema(req) => {
            let value = super::update::request_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::UpdateRequestSchema(value)))
        }
        Request::UpdateResponseSchema(req) => {
            let value = super::update::response_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::UpdateResponseSchema(value)))
        }
        Request::Viewer(req) => {
            let inner = super::viewer::execute(global, scoped, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Viewer)))
        }
    };
    Ok(stream)
}
