//! `agents logs` — CLI-side dispatch for the logs subtree.
//! `open` (look up one row), `list` (stream rows: `--all`/`--pending`),
//! `subscribe` (live stream of new rows).

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::agents::logs::{Request, ResponseItem};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub mod list;
pub mod open;
pub mod subscribe;
pub mod token_usage;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

fn once<T: Send + 'static>(
    item: Result<T, Error>,
) -> Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>> {
    Box::pin(futures::stream::once(async move { item }))
}

pub async fn execute(global: &GlobalContext, scoped: &ScopedContext, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::Open(req) => {
            let value = open::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::Open(value)))
        }
        Request::OpenRequestSchema(req) => {
            let value = open::request_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::OpenRequestSchema(value)))
        }
        Request::OpenResponseSchema(req) => {
            let value = open::response_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::OpenResponseSchema(value)))
        }
        Request::List(req) => {
            let inner = list::execute(global, scoped, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::List)))
        }
        Request::ListRequestSchema(req) => {
            let value = list::request_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::ListRequestSchema(value)))
        }
        Request::ListResponseSchema(req) => {
            let value = list::response_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::ListResponseSchema(value)))
        }
        Request::Subscribe(req) => {
            let inner = subscribe::execute(global, scoped, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Subscribe)))
        }
        Request::SubscribeRequestSchema(req) => {
            let value = subscribe::request_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::SubscribeRequestSchema(value)))
        }
        Request::SubscribeResponseSchema(req) => {
            let value = subscribe::response_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::SubscribeResponseSchema(value)))
        }
        Request::TokenUsage(req) => {
            let inner = token_usage::execute(global, scoped, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::TokenUsage)))
        }
    };
    Ok(stream)
}
