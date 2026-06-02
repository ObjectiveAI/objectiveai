//! `functions executions create` sub-tier. Standard and SwissSystem are
//! chunk-or-id streaming leaves; their bare-naked `execute` returns
//! `Stream<ResponseItem>` and the inner stream is mapped into the
//! tier's `ResponseItem` directly.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::functions::executions::create::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

pub mod standard;
pub mod swiss_system;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

fn once<T: Send + 'static>(
    item: Result<T, Error>,
) -> Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>> {
    Box::pin(futures::stream::once(async move { item }))
}

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::Standard(req) => {
            let inner = standard::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Standard)))
        }
        Request::StandardRequestSchema(req) => {
            let value = standard::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::StandardRequestSchema(value)))
        }
        Request::StandardResponseSchema(req) => {
            let value = standard::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::StandardResponseSchema(value)))
        }
        Request::SwissSystem(req) => {
            let inner = swiss_system::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::SwissSystem)))
        }
        Request::SwissSystemRequestSchema(req) => {
            let value = swiss_system::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::SwissSystemRequestSchema(value)))
        }
        Request::SwissSystemResponseSchema(req) => {
            let value = swiss_system::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::SwissSystemResponseSchema(value)))
        }
    };
    Ok(stream)
}
