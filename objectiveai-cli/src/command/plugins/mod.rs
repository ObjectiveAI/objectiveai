//! `plugins` tier dispatch. Mirrors
//! `objectiveai-sdk-rs/src/cli/command/plugins/mod.rs`. `install`,
//! `list`, and `run` are streaming; `get` is unary. `install` is a
//! sub-tier with its own dispatcher under `install/mod.rs`.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::plugins::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

pub mod daemon;
pub mod get;
pub mod install;
pub mod list;
pub mod run;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

fn once<T: Send + 'static>(
    item: Result<T, Error>,
) -> Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>> {
    Box::pin(futures::stream::once(async move { item }))
}

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::Daemon(req) => {
            let inner = daemon::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Daemon)))
        }
        Request::Get(req) => {
            let value = get::execute(ctx, req).await?;
            once(Ok(ResponseItem::Get(value)))
        }
        Request::GetRequestSchema(req) => {
            let value = get::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::GetRequestSchema(value)))
        }
        Request::GetResponseSchema(req) => {
            let value = get::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::GetResponseSchema(value)))
        }
        Request::Install(req) => {
            let inner = install::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Install)))
        }
        Request::List(req) => {
            let inner = list::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::List)))
        }
        Request::ListRequestSchema(req) => {
            let value = list::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::ListRequestSchema(value)))
        }
        Request::ListResponseSchema(req) => {
            let value = list::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::ListResponseSchema(value)))
        }
        Request::Run(req) => {
            let inner = run::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Run)))
        }
        Request::RunRequestSchema(req) => {
            let value = run::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::RunRequestSchema(value)))
        }
        Request::RunResponseSchema(req) => {
            let value = run::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::RunResponseSchema(value)))
        }
    };
    Ok(stream)
}
