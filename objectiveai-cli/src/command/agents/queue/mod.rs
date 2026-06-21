//! `agents queue` — CLI-side dispatch for the queue subtree.
//! `open` (fetch one content piece), `list` (stream pending prompts),
//! `delete` (drop one row), `deliver` (wake pending descendants).
//! Enqueue is gone — `agents message` handles it under the hood.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::agents::queue::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

pub mod delete;
pub mod deliver;
pub mod list;
pub mod open;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

fn once<T: Send + 'static>(
    item: Result<T, Error>,
) -> Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>> {
    Box::pin(futures::stream::once(async move { item }))
}

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::Delete(req) => {
            let value = delete::execute(ctx, req).await?;
            once(Ok(ResponseItem::Delete(value)))
        }
        Request::DeleteRequestSchema(req) => {
            let value = delete::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::DeleteRequestSchema(value)))
        }
        Request::DeleteResponseSchema(req) => {
            let value = delete::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::DeleteResponseSchema(value)))
        }
        Request::Deliver(req) => {
            let inner = deliver::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Deliver)))
        }
        Request::DeliverRequestSchema(req) => {
            let value = deliver::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::DeliverRequestSchema(value)))
        }
        Request::DeliverResponseSchema(req) => {
            let value = deliver::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::DeliverResponseSchema(value)))
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
        Request::Open(req) => {
            let value = open::execute(ctx, req).await?;
            once(Ok(ResponseItem::Open(value)))
        }
        Request::OpenRequestSchema(req) => {
            let value = open::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::OpenRequestSchema(value)))
        }
        Request::OpenResponseSchema(req) => {
            let value = open::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::OpenResponseSchema(value)))
        }
    };
    Ok(stream)
}
