//! `agents queue` — CLI-side dispatch for the queue subtree. `add`
//! is gone — `agents message` now handles enqueue under the hood.
//!
//! NOTE: scheduled for an upcoming refactor — the internal leaves
//! still reflect the prior shape and are intentionally left untouched
//! by the current rename pass.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::agents::queue::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

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
