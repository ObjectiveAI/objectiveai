//! `agents message-queue` Ã¢â‚¬â€ CLI-side dispatch for the queue subtree.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::agents::message_queue::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

pub mod add;
pub mod delete;
pub mod read;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

fn once<T: Send + 'static>(
    item: Result<T, Error>,
) -> Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>> {
    Box::pin(futures::stream::once(async move { item }))
}

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::Add(req) => {
            let value = add::execute(ctx, req).await?;
            once(Ok(ResponseItem::Add(value)))
        }
        Request::AddRequestSchema(req) => {
            let value = add::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::AddRequestSchema(value)))
        }
        Request::AddResponseSchema(req) => {
            let value = add::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::AddResponseSchema(value)))
        }
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
        Request::Read(req) => {
            let inner = read::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Read)))
        }
    };
    Ok(stream)
}
